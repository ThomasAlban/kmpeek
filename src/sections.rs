use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

// every struct here should have a new() function that takes a reader and returns a new struct, and a write() function that writes itself as bytes to the writer
pub trait KMPData {
    fn new(rdr: impl Read) -> io::Result<Self>
    where
        Self: Sized;

    fn write<T>(&self, wtr: T) -> io::Result<T>
    where
        T: Write;
}

/// stores all the data of the KMP file
#[derive(Serialize, Deserialize)]
pub struct KMP {
    header: Header,
    ktpt: Section<KTPT>,
    enpt: Section<ENPT>,
    enph: Section<Path>,
    itpt: Section<ITPT>,
    itph: Section<Path>,
    ckpt: Section<CKPT>,
    ckph: Section<Path>,
    gobj: Section<GOBJ>,
    poti: Section<POTI>,
    area: Section<AREA>,
    came: Section<CAME>,
    jgpt: Section<JGPT>,
    cnpt: Section<CNPT>,
    mspt: Section<MSPT>,
    stgi: Section<STGI>,
}
impl KMPData for KMP {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let header = Header::new(&mut rdr)?;

        let ktpt = Section::<KTPT>::new(&mut rdr)?;
        let enpt = Section::<ENPT>::new(&mut rdr)?;
        let enph = Section::<Path>::new(&mut rdr)?;
        let itpt = Section::<ITPT>::new(&mut rdr)?;
        let itph = Section::<Path>::new(&mut rdr)?;
        let ckpt = Section::<CKPT>::new(&mut rdr)?;
        let ckph = Section::<Path>::new(&mut rdr)?;
        let gobj = Section::<GOBJ>::new(&mut rdr)?;
        let poti = Section::<POTI>::new(&mut rdr)?;
        let area = Section::<AREA>::new(&mut rdr)?;
        let came = Section::<CAME>::new(&mut rdr)?;
        let jgpt = Section::<JGPT>::new(&mut rdr)?;
        let cnpt = Section::<CNPT>::new(&mut rdr)?;
        let mspt = Section::<MSPT>::new(&mut rdr)?;
        let stgi = Section::<STGI>::new(&mut rdr)?;

        Ok(KMP {
            header,
            ktpt,
            enpt,
            enph,
            itpt,
            itph,
            ckpt,
            ckph,
            gobj,
            poti,
            area,
            came,
            jgpt,
            cnpt,
            mspt,
            stgi,
        })
    }
    fn write<T>(&self, wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        let mut wtr = self.header.write(wtr)?;
        wtr = self.ktpt.write(wtr)?;
        wtr = self.enpt.write(wtr)?;
        wtr = self.enph.write(wtr)?;
        wtr = self.itpt.write(wtr)?;
        wtr = self.itph.write(wtr)?;
        wtr = self.ckpt.write(wtr)?;
        wtr = self.ckph.write(wtr)?;
        wtr = self.gobj.write(wtr)?;
        wtr = self.poti.write(wtr)?;
        wtr = self.area.write(wtr)?;
        wtr = self.came.write(wtr)?;
        wtr = self.jgpt.write(wtr)?;
        wtr = self.cnpt.write(wtr)?;
        wtr = self.mspt.write(wtr)?;
        wtr = self.stgi.write(wtr)?;
        Ok(wtr)
    }
}

/// The header, which contains general information about the KMP
#[derive(Serialize, Deserialize)]
pub struct Header {
    file_magic: String,
    file_len: u32,
    num_sections: u16,
    header_len: u16,
    version_num: u32,
    section_offsets: [u32; 15],
}
impl KMPData for Header {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        // get the first 4 bytes of the file for file magic
        let mut file_magic = [0u8; 4];
        for i in 0..4 {
            let byte = rdr.read_u8()?;
            file_magic[i] = byte;
            if file_magic[i] != b"RKMD"[i] {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid file magic",
                ));
            }
        }
        let file_magic = String::from_utf8(file_magic.to_vec());
        if let Err(_) = file_magic {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid file magic",
            ));
        }
        let file_magic = file_magic.unwrap();
        let file_len = rdr.read_u32::<BE>()?;
        // check that the number of sections is 15
        let num_sections = rdr.read_u16::<BE>()?;
        if num_sections != 15 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid number of sections in header, expected 15 but found {}",
                    num_sections
                ),
            ));
        }
        let header_len = rdr.read_u16::<BE>()?;
        let version_num = rdr.read_u32::<BE>()?;
        let mut section_offsets = [0u32; 15];
        for i in 0..15 {
            let byte = rdr.read_u32::<BE>()?;
            section_offsets[i] = byte;
        }
        Ok(Header {
            file_magic,
            file_len,
            num_sections,
            header_len,
            version_num,
            section_offsets,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write(self.file_magic.as_bytes())?;
        wtr.write_u32::<BE>(self.file_len)?;
        wtr.write_u16::<BE>(self.num_sections)?;
        wtr.write_u16::<BE>(self.header_len)?;
        wtr.write_u32::<BE>(self.version_num)?;
        for e in self.section_offsets {
            wtr.write_u32::<BE>(e)?;
        }
        Ok(wtr)
    }
}

/// Each section has a header containing its info (like the name and number of entries)
#[derive(Serialize, Deserialize)]
struct SectionHeader {
    section_name: String,
    num_entries: u16,
    /// The POTI section stores the total number of points of all routes here. The CAME section stores different values. For all other sections, the value is 0 (padding).
    additional_value: u16,
}
impl KMPData for SectionHeader {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut section_name = [0u8; 4];
        for i in 0..4 {
            let byte = rdr.read_u8()?;
            section_name[i] = byte;
        }
        let section_name = String::from_utf8(section_name.to_vec());
        if let Err(_) = section_name {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid section name",
            ));
        }
        let section_name = section_name.unwrap();
        let num_entries = rdr.read_u16::<BE>()?;
        let additional_value = rdr.read_u16::<BE>()?;
        Ok(SectionHeader {
            section_name,
            num_entries,
            additional_value,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write(self.section_name.as_bytes())?;
        wtr.write_u16::<BE>(self.num_entries)?;
        wtr.write_u16::<BE>(self.additional_value)?;
        Ok(wtr)
    }
}

/// A generic type for a section of a KMP - each section contains a header, and a number of entries.
#[derive(Serialize, Deserialize)]
pub struct Section<T>
where
    T: KMPData,
{
    section_header: SectionHeader,
    entries: Vec<T>,
}
impl<T> KMPData for Section<T>
where
    T: KMPData,
{
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        // make a new section header object
        let section_header = SectionHeader::new(&mut rdr)?;
        // for each entry in the section, make a new entry object
        let mut entries = Vec::new();
        for _ in 0..section_header.num_entries {
            let entry = T::new(&mut rdr)?;
            entries.push(entry);
        }
        // return the section object
        Ok(Section {
            section_header,
            entries,
        })
    }
    fn write<U>(&self, wtr: U) -> io::Result<U>
    where
        U: Write,
    {
        let mut wtr = self.section_header.write(wtr)?;
        for e in &self.entries {
            wtr = e.write(wtr)?;
        }
        Ok(wtr)
    }
}

/// Sections of the KMP such as ENPH (enemy paths), ITPH (item paths) all have the same data structure, so all use this Path struct.
#[derive(Serialize, Deserialize)]
pub struct Path {
    start: u8,
    group_length: u8,
    prev_group: [u8; 6],
    next_group: [u8; 6],
}
impl KMPData for Path {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let start = rdr.read_u8()?;
        let group_length = rdr.read_u8()?;
        let mut prev_group = [0u8; 6];
        for i in 0..6 {
            let byte = rdr.read_u8()?;
            prev_group[i] = byte;
        }
        let mut next_group = [0u8; 6];
        for i in 0..6 {
            let byte = rdr.read_u8()?;
            next_group[i] = byte;
        }
        // padding
        rdr.read_u16::<BE>()?;
        Ok(Path {
            start,
            group_length,
            prev_group,
            next_group,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_u8(self.start)?;
        wtr.write_u8(self.group_length)?;
        wtr.write_all(self.prev_group.as_slice())?;
        wtr.write_all(self.next_group.as_slice())?;
        // padding
        wtr.write(0u16.to_be_bytes().as_slice())?;
        Ok(wtr)
    }
}

/// The KTPT (kart point) section describes kart points; the starting position for racers.
#[derive(Serialize, Deserialize)]
pub struct KTPT {
    position: [f32; 3],
    rotation: [f32; 3],
    player_index: i16,
}
impl KMPData for KTPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let mut rotation = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            rotation[i] = byte;
        }
        let player_index = rdr.read_i16::<BE>()?;
        // padding
        rdr.read_u16::<BE>()?;
        Ok(KTPT {
            position,
            rotation,
            player_index,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.rotation {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_i16::<BE>(self.player_index)?;
        // padding
        wtr.write_u16::<BE>(0)?;
        Ok(wtr)
    }
}

/// The ENPT (enemy point) section describes enemy points; the routes of CPU racers. The CPU racers attempt to follow the path described by each group of points (as determined by ENPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Serialize, Deserialize)]
pub struct ENPT {
    position: [f32; 3],
    leniency: f32,
    setting_1: u16,
    setting_2: u8,
    setting_3: u8,
}
impl KMPData for ENPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let leniency = rdr.read_f32::<BE>()?;
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u8()?;
        let setting_3 = rdr.read_u8()?;
        Ok(ENPT {
            position,
            leniency,
            setting_1,
            setting_2,
            setting_3,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_f32::<BE>(self.leniency)?;
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u8(self.setting_2)?;
        wtr.write_u8(self.setting_3)?;
        Ok(wtr)
    }
}

/// The ITPT (item point) section describes item points; the Red Shell and Bullet Bill routes. The items attempt to follow the path described by each group of points (as determined by ITPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Serialize, Deserialize)]
pub struct ITPT {
    position: [f32; 3],
    bullet_bill_control: f32,
    setting_1: u16,
    setting_2: u16,
}
impl KMPData for ITPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let bullet_bill_control = rdr.read_f32::<BE>()?;
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u16::<BE>()?;
        Ok(ITPT {
            position,
            bullet_bill_control,
            setting_1,
            setting_2,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_f32::<BE>(self.bullet_bill_control)?;
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u16::<BE>(self.setting_2)?;
        Ok(wtr)
    }
}

/// The CKPT (checkpoint) section describes checkpoints; the routes players must follow to count laps. The racers must follow the path described by each group of points (as determined by CKPH). More than 0xFF (255) entries are possible if the last group begins at index ≤254. This is not recommended because Lakitu will always appear on-screen.
#[derive(Serialize, Deserialize)]
pub struct CKPT {
    cp_left: [f32; 2],
    cp_right: [f32; 2],
    respawn_pos: u8,
    cp_type: u8,
    prev_cp: u8,
    next_cp: u8,
}
impl KMPData for CKPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let cp_left = [rdr.read_f32::<BE>()?, rdr.read_f32::<BE>()?];
        let cp_right = [rdr.read_f32::<BE>()?, rdr.read_f32::<BE>()?];
        let respawn_pos = rdr.read_u8()?;
        let cp_type = rdr.read_u8()?;
        let prev_cp = rdr.read_u8()?;
        let next_cp = rdr.read_u8()?;
        Ok(CKPT {
            cp_left,
            cp_right,
            respawn_pos,
            cp_type,
            prev_cp,
            next_cp,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_f32::<BE>(self.cp_left[0])?;
        wtr.write_f32::<BE>(self.cp_left[1])?;
        wtr.write_f32::<BE>(self.cp_right[0])?;
        wtr.write_f32::<BE>(self.cp_right[1])?;
        wtr.write_u8(self.respawn_pos)?;
        wtr.write_u8(self.cp_type)?;
        wtr.write_u8(self.prev_cp)?;
        wtr.write_u8(self.next_cp)?;
        Ok(wtr)
    }
}

/// The GOBJ (geo object) section describes objects; things such as item boxes, pipes and also controlled objects such as sound triggers.
#[derive(Serialize, Deserialize)]
pub struct GOBJ {
    object_id: u16,
    /// this is part of the extended presence flags, but the value must be 0 if the object does not use this extension
    padding: u16,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    route: u16,
    settings: [u16; 8],
    presence_flags: u16,
}
impl KMPData for GOBJ {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let object_id = rdr.read_u16::<BE>()?;
        let padding = rdr.read_u16::<BE>()?;
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let mut rotation = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            rotation[i] = byte;
        }
        let mut scale = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            scale[i] = byte;
        }
        let route = rdr.read_u16::<BE>()?;
        let mut settings = [0u16; 8];
        for i in 0..8 {
            let byte = rdr.read_u16::<BE>()?;
            settings[i] = byte;
        }
        let presence_flags = rdr.read_u16::<BE>()?;
        Ok(GOBJ {
            object_id,
            padding,
            position,
            rotation,
            scale,
            route,
            settings,
            presence_flags,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_u16::<BE>(self.object_id)?;
        wtr.write_u16::<BE>(self.padding)?;
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.rotation {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.scale {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.route)?;
        for e in self.settings {
            wtr.write_u16::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.presence_flags)?;
        Ok(wtr)
    }
}

/// Each POTI entry can contain a number of POTI entries/points.
#[derive(Serialize, Deserialize)]
struct POTIPoint {
    position: [f32; 3],
    setting_1: u16,
    setting_2: u16,
}
impl KMPData for POTIPoint {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u16::<BE>()?;
        Ok(POTIPoint {
            position,
            setting_1,
            setting_2,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u16::<BE>(self.setting_2)?;
        Ok(wtr)
    }
}

/// The POTI (point information) section describes routes; these are routes for many things including cameras and objects.
#[derive(Serialize, Deserialize)]
pub struct POTI {
    num_points: u16,
    setting_1: u8,
    setting_2: u8,
    routes: Vec<POTIPoint>,
}
impl KMPData for POTI {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let num_points = rdr.read_u16::<BE>()?;
        let setting_1 = rdr.read_u8()?;
        let setting_2 = rdr.read_u8()?;
        let mut routes = Vec::new();
        for _ in 0..num_points {
            routes.push(POTIPoint::new(&mut rdr)?);
        }
        Ok(POTI {
            num_points,
            setting_1,
            setting_2,
            routes,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_u16::<BE>(self.num_points)?;
        wtr.write_u8(self.setting_1)?;
        wtr.write_u8(self.setting_2)?;
        for e in &self.routes {
            wtr = e.write(wtr)?;
        }
        Ok(wtr)
    }
}

/// The AREA (area) section describes areas; used to determine which camera to use, for example. The size is 5000 for both the positive and negative sides of the X and Z-axes, and 10000 for only the positive side of the Y-axis.
#[derive(Serialize, Deserialize)]
pub struct AREA {
    shape: u8,
    kind: u8,
    came_index: u8,
    priority: u8,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    setting_1: u16,
    setting_2: u16,
    route: u8,
    enpt_id: u8,
}
impl KMPData for AREA {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let shape = rdr.read_u8()?;
        let kind = rdr.read_u8()?;
        let came_index = rdr.read_u8()?;
        let priority = rdr.read_u8()?;
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let mut rotation = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            rotation[i] = byte;
        }
        let mut scale = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            scale[i] = byte;
        }
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u16::<BE>()?;
        let route = rdr.read_u8()?;
        let enpt_id = rdr.read_u8()?;
        // padding
        rdr.read_u16::<BE>()?;
        Ok(AREA {
            shape,
            kind,
            came_index,
            priority,
            position,
            rotation,
            scale,
            setting_1,
            setting_2,
            route,
            enpt_id,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_u8(self.shape)?;
        wtr.write_u8(self.kind)?;
        wtr.write_u8(self.came_index)?;
        wtr.write_u8(self.priority)?;
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.rotation {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.scale {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u16::<BE>(self.setting_2)?;
        wtr.write_u8(self.route)?;
        wtr.write_u8(self.enpt_id)?;
        // padding
        wtr.write_u16::<BE>(0)?;
        Ok(wtr)
    }
}

/// The CAME (camera) section describes cameras; used to determine cameras for starting routes, Time Trial pans, etc.
#[derive(Serialize, Deserialize)]
pub struct CAME {
    kind: u8,
    next_index: u8,
    shake: u8,
    route: u8,
    point_velocity: u16,
    zoom_velocity: u16,
    view_velocity: u16,
    start: u8,
    movie: u8,
    position: [f32; 3],
    rotation: [f32; 3],
    zoom_start: f32,
    zoom_end: f32,
    view_start: [f32; 3],
    view_end: [f32; 3],
    time: f32,
}
impl KMPData for CAME {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let kind = rdr.read_u8()?;
        let next_index = rdr.read_u8()?;
        let shake = rdr.read_u8()?;
        let route = rdr.read_u8()?;
        let point_velocity = rdr.read_u16::<BE>()?;
        let zoom_velocity = rdr.read_u16::<BE>()?;
        let view_velocity = rdr.read_u16::<BE>()?;
        let start = rdr.read_u8()?;
        let movie = rdr.read_u8()?;
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let mut rotation = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            rotation[i] = byte;
        }
        let zoom_start = rdr.read_f32::<BE>()?;
        let zoom_end = rdr.read_f32::<BE>()?;
        let mut view_start = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            view_start[i] = byte;
        }
        let mut view_end = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            view_end[i] = byte;
        }
        let time = rdr.read_f32::<BE>()?;
        Ok(CAME {
            kind,
            next_index,
            shake,
            route,
            point_velocity,
            zoom_velocity,
            view_velocity,
            start,
            movie,
            position,
            rotation,
            zoom_start,
            zoom_end,
            view_start,
            view_end,
            time,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_u8(self.kind)?;
        wtr.write_u8(self.next_index)?;
        wtr.write_u8(self.shake)?;
        wtr.write_u8(self.route)?;
        wtr.write_u16::<BE>(self.point_velocity)?;
        wtr.write_u16::<BE>(self.zoom_velocity)?;
        wtr.write_u16::<BE>(self.view_velocity)?;
        wtr.write_u8(self.start)?;
        wtr.write_u8(self.movie)?;
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.rotation {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_f32::<BE>(self.zoom_start)?;
        wtr.write_f32::<BE>(self.zoom_end)?;
        for e in self.view_start {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.view_end {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_f32::<BE>(self.time)?;
        Ok(wtr)
    }
}

/// The JGPT (jugem point) section describes "Jugem" points; the respawn positions. The index is relevant for the link of the CKPT section.
#[derive(Serialize, Deserialize)]
pub struct JGPT {
    position: [f32; 3],
    rotation: [f32; 3],
    respawn_id: u16,
    extra_data: i16,
}
impl KMPData for JGPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let mut rotation = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            rotation[i] = byte;
        }
        let respawn_id = rdr.read_u16::<BE>()?;
        let extra_data = rdr.read_i16::<BE>()?;
        Ok(JGPT {
            position,
            rotation,
            respawn_id,
            extra_data,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.rotation {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.respawn_id)?;
        wtr.write_i16::<BE>(self.extra_data)?;
        Ok(wtr)
    }
}

/// The CNPT (cannon point) section describes cannon points; the cannon target positions.
#[derive(Serialize, Deserialize)]
pub struct CNPT {
    postition: [f32; 3],
    angle: [f32; 3],
    id: u16,
    shoot_effect: i16,
}
impl KMPData for CNPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut postition = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            postition[i] = byte;
        }
        let mut angle = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            angle[i] = byte;
        }
        let id = rdr.read_u16::<BE>()?;
        let shoot_effect = rdr.read_i16::<BE>()?;
        Ok(CNPT {
            postition,
            angle,
            id,
            shoot_effect,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.postition {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.angle {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.id)?;
        wtr.write_i16::<BE>(self.shoot_effect)?;
        Ok(wtr)
    }
}

/// The MSPT (mission success point) section describes end positions. After battles and tournaments have ended, the players are placed on this point(s).
#[derive(Serialize, Deserialize)]
pub struct MSPT {
    position: [f32; 3],
    angle: [f32; 3],
    id: u16,
    unknown: u16,
}
impl KMPData for MSPT {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let mut position = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            position[i] = byte;
        }
        let mut angle = [0f32; 3];
        for i in 0..3 {
            let byte = rdr.read_f32::<BE>()?;
            angle[i] = byte;
        }
        let id = rdr.read_u16::<BE>()?;
        let unknown = rdr.read_u16::<BE>()?;
        Ok(MSPT {
            position,
            angle,
            id,
            unknown,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        for e in self.position {
            wtr.write_f32::<BE>(e)?;
        }
        for e in self.angle {
            wtr.write_f32::<BE>(e)?;
        }
        wtr.write_u16::<BE>(self.id)?;
        wtr.write_u16::<BE>(self.unknown)?;
        Ok(wtr)
    }
}

/// The STGI (stage info) section describes stage information; information about a track.
#[derive(Serialize, Deserialize)]
pub struct STGI {
    lap_count: u8,
    pole_pos: u8,
    driver_distance: u8,
    lens_flare_flashing: u8,
    flare_color: u32,
    flare_transparency: u8,
    /// Always 0 in Nintendo tracks. This is for the speed modifier cheat code.
    speed_mod: f32,
}
impl KMPData for STGI {
    fn new(mut rdr: impl Read) -> io::Result<Self> {
        let lap_count = rdr.read_u8()?;
        let pole_pos = rdr.read_u8()?;
        let driver_distance = rdr.read_u8()?;
        let lens_flare_flashing = rdr.read_u8()?;
        let flare_color = rdr.read_u32::<BE>()?;
        let flare_transparency = rdr.read_u8()?;
        // padding
        rdr.read_u8()?;
        let mut speed_mod = [0u8; 4];
        for i in 0..2 {
            speed_mod[i] = rdr.read_u8()?;
        }
        let speed_mod = f32::from_be_bytes(speed_mod);
        Ok(STGI {
            lap_count,
            pole_pos,
            driver_distance,
            lens_flare_flashing,
            flare_color,
            flare_transparency,
            speed_mod,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_u8(self.lap_count)?;
        wtr.write_u8(self.pole_pos)?;
        wtr.write_u8(self.driver_distance)?;
        wtr.write_u8(self.lens_flare_flashing)?;
        wtr.write_u32::<BE>(self.flare_color)?;
        wtr.write_u8(self.flare_transparency)?;
        // padding
        wtr.write_u8(0)?;
        let bytes = self.speed_mod.to_be_bytes();
        wtr.write_u8(bytes[0])?;
        wtr.write_u8(bytes[1])?;
        Ok(wtr)
    }
}
