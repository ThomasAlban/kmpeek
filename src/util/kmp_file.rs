use crate::util::read_write_arrays::{ReadArrays, WriteArrays};
use bevy::{math::Vec3, prelude::*};
use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Seek, Write};

// every struct here should have a read() function that takes a reader and returns a read struct, and a write() function that writes itself as bytes to the writer
pub trait KmpData {
    fn read(rdr: impl Read) -> io::Result<Self>
    where
        Self: Sized;
    fn write<T>(&self, wtr: T) -> io::Result<T>
    where
        T: Write + Read + Seek;
}

pub trait KmpSectionName {
    fn section_name() -> String;
}
pub trait KmpPathSectionName {
    fn path_section_name() -> String;
}

macro_rules! invalid_data_error {
    ($msg:expr) => {
        Err(io::Error::new(io::ErrorKind::InvalidData, $msg))
    };
}

/// stores all the data of the KMP file
#[derive(Debug, Serialize, Deserialize, Resource, Clone, Reflect)]
pub struct Kmp {
    pub header: Header,
    pub ktpt: Section<Ktpt>,
    pub enpt: Section<Enpt>,
    pub enph: Section<PathGroup>,
    pub itpt: Section<Itpt>,
    pub itph: Section<PathGroup>,
    pub ckpt: Section<Ckpt>,
    pub ckph: Section<PathGroup>,
    pub gobj: Section<Gobj>,
    pub poti: Section<Poti>,
    pub area: Section<Area>,
    pub came: Section<Came>,
    pub jgpt: Section<Jgpt>,
    pub cnpt: Section<Cnpt>,
    pub mspt: Section<Mspt>,
    pub stgi: Section<Stgi>,
}
impl Kmp {
    /// Read a KMP from an object that implements Read, returning either a KMP object or an error.
    pub fn read(mut rdr: impl Read) -> io::Result<Self> {
        let header = Header::read(&mut rdr)?;

        let ktpt = Section::<Ktpt>::read(&mut rdr)?;
        let enpt = Section::<Enpt>::read(&mut rdr)?;
        let enph = Section::<PathGroup>::read(&mut rdr)?;
        let itpt = Section::<Itpt>::read(&mut rdr)?;
        let itph = Section::<PathGroup>::read(&mut rdr)?;
        let ckpt = Section::<Ckpt>::read(&mut rdr)?;
        let ckph = Section::<PathGroup>::read(&mut rdr)?;
        let gobj = Section::<Gobj>::read(&mut rdr)?;
        let poti = Section::<Poti>::read(&mut rdr)?;
        let area = Section::<Area>::read(&mut rdr)?;
        let came = Section::<Came>::read(&mut rdr)?;
        let jgpt = Section::<Jgpt>::read(&mut rdr)?;
        let cnpt = Section::<Cnpt>::read(&mut rdr)?;
        let mspt = Section::<Mspt>::read(&mut rdr)?;
        let stgi = Section::<Stgi>::read(&mut rdr)?;

        Ok(Kmp {
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
    /// Write the KMP object to an object that implements Write.
    #[allow(dead_code)]
    pub fn write<T>(&mut self, mut wtr: T) -> io::Result<T>
    where
        T: Write + Read + Seek,
    {
        let header_len: u32 = 0x4C;

        // write temporary padding which will later be replaced by the header once we know the file size and section offsets
        for _ in 0..header_len {
            wtr.write_u8(0)?;
        }
        // for each section, we set the section offset in the header to its position relative to the end of the header
        // then we write the section to the writer
        macro_rules! section {
            ($section:ident, $i:expr) => {
                self.header.section_offsets[$i] = wtr.stream_position()? as u32 - header_len;
                wtr = self.$section.write(wtr)?;
            };
        }
        section!(ktpt, 0);
        section!(enpt, 1);
        section!(enph, 2);
        section!(itpt, 3);
        section!(itph, 4);
        section!(ckpt, 5);
        section!(ckph, 6);
        section!(gobj, 7);
        section!(poti, 8);
        section!(area, 9);
        section!(came, 10);
        section!(jgpt, 11);
        section!(cnpt, 12);
        section!(mspt, 13);
        section!(stgi, 14);

        // set the header file length to where we currently are (which is the end of the file)
        self.header.file_len = wtr.stream_position()? as u32;
        // go back to the beginning and write the file header
        wtr.rewind()?;
        wtr = self.header.write(wtr)?;

        Ok(wtr)
    }
}

/// The header, which contains general information about the KMP
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Header {
    file_magic: String,
    file_len: u32,
    num_sections: u16,
    header_len: u16,
    version_num: u32,
    section_offsets: [u32; 15],
}
impl KmpData for Header {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        // get the first 4 bytes of the file for file magic
        let file_magic = rdr.read_array::<u8, 4>()?;
        if &file_magic != b"RKMD" {
            return invalid_data_error!("Invalid file magic");
        }
        // convert file magic to string
        let Ok(file_magic) = String::from_utf8(file_magic.to_vec()) else {
            return invalid_data_error!("Invalid file magic");
        };
        let file_len = rdr.read_u32::<BE>()?;
        // check that the number of sections is 15
        let num_sections = rdr.read_u16::<BE>()?;
        if num_sections != 15 {
            return invalid_data_error!("Number of sections not equal to 15");
        }
        let header_len = rdr.read_u16::<BE>()?;
        let version_num = rdr.read_u32::<BE>()?;
        let section_offsets = rdr.read_array::<u32, 15>()?;
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
        let bytes_written = wtr.write(self.file_magic.as_bytes())?;
        if bytes_written != self.file_magic.len() {
            return invalid_data_error!("Could not write file magic");
        }
        wtr.write_u32::<BE>(self.file_len)?;
        wtr.write_u16::<BE>(self.num_sections)?;
        wtr.write_u16::<BE>(self.header_len)?;
        wtr.write_u32::<BE>(self.version_num)?;
        wtr.write_array(self.section_offsets)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Header {
    fn section_name() -> String {
        "header".into()
    }
}

/// Each section has a header containing its info (like the name and number of entries)
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
struct SectionHeader {
    section_name: String,
    num_entries: u16,
    /// The POTI section stores the total number of points of all routes here. The CAME section stores different values. For all other sections, the value is 0 (padding).
    additional_value: u16,
}
impl KmpData for SectionHeader {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let section_name = rdr.read_array::<u8, 4>()?;
        let Ok(section_name) = String::from_utf8(section_name.to_vec()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid section name",
            ));
        };
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
        wtr.write_all(self.section_name.as_bytes())?;
        wtr.write_u16::<BE>(self.num_entries)?;
        wtr.write_u16::<BE>(self.additional_value)?;
        Ok(wtr)
    }
}

/// A generic type for a section of a KMP - each section contains a header, and a number of entries.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Section<T>
where
    T: KmpData + Reflect,
{
    section_header: SectionHeader,
    pub entries: Vec<T>,
}
impl<T> KmpData for Section<T>
where
    T: KmpData + Reflect,
{
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        // make a read section header object
        let section_header = SectionHeader::read(&mut rdr)?;
        // for each entry in the section, make a read entry object
        let mut entries = Vec::new();
        for _ in 0..section_header.num_entries {
            let entry = T::read(&mut rdr)?;
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
        U: Write + Read + Seek,
    {
        let mut wtr = self.section_header.write(wtr)?;
        for e in &self.entries {
            wtr = e.write(wtr)?;
        }
        Ok(wtr)
    }
}

/// The KTPT (kart point) section describes kart points; the starting position for racers.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Ktpt {
    pub position: Vec3,
    pub rotation: Vec3,
    pub player_index: i16,
}
impl KmpData for Ktpt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let rotation = rdr.read_vec3()?;
        let player_index = rdr.read_i16::<BE>()?;
        rdr.read_u16::<BE>()?;
        Ok(Ktpt {
            position,
            rotation,
            player_index,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.rotation)?;
        wtr.write_i16::<BE>(self.player_index)?;
        wtr.write_u16::<BE>(0)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Ktpt {
    fn section_name() -> String {
        "ktpt".into()
    }
}

/// The ENPT (enemy point) section describes enemy points; the routes of CPU racers. The CPU racers attempt to follow the path described by each group of points (as determined by ENPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Enpt {
    pub position: Vec3,
    pub leniency: f32,
    pub setting_1: u16,
    pub setting_2: u8,
    pub setting_3: u8,
}
impl KmpData for Enpt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let leniency = rdr.read_f32::<BE>()?;
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u8()?;
        let setting_3 = rdr.read_u8()?;
        Ok(Enpt {
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
        wtr.write_vec3(self.position)?;
        wtr.write_f32::<BE>(self.leniency)?;
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u8(self.setting_2)?;
        wtr.write_u8(self.setting_3)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Enpt {
    fn section_name() -> String {
        "enpt".into()
    }
}
impl KmpPathSectionName for Enpt {
    fn path_section_name() -> String {
        "enph".into()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct PathGroup {
    pub start: u8,
    pub group_length: u8,
    pub prev_group: [u8; 6],
    pub next_group: [u8; 6],
}
impl KmpData for PathGroup {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let start = rdr.read_u8()?;
        let group_length = rdr.read_u8()?;
        let prev_group = rdr.read_array::<u8, 6>()?;
        let next_group = rdr.read_array::<u8, 6>()?;
        rdr.read_u16::<BE>()?;
        Ok(PathGroup {
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
        wtr.write_array(self.prev_group)?;
        wtr.write_array(self.next_group)?;
        wtr.write_u16::<BE>(0)?;
        Ok(wtr)
    }
}

/// The ITPT (item point) section describes item points; the Red Shell and Bullet Bill routes. The items attempt to follow the path described by each group of points (as determined by ITPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect)]
pub struct Itpt {
    pub position: Vec3,
    pub bullet_bill_control: f32,
    pub setting_1: u16,
    pub setting_2: u16,
}
impl KmpData for Itpt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let bullet_bill_control = rdr.read_f32::<BE>()?;
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u16::<BE>()?;
        Ok(Itpt {
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
        wtr.write_vec3(self.position)?;
        wtr.write_f32::<BE>(self.bullet_bill_control)?;
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u16::<BE>(self.setting_2)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Itpt {
    fn section_name() -> String {
        "itpt".into()
    }
}
impl KmpPathSectionName for Itpt {
    fn path_section_name() -> String {
        "itph".into()
    }
}

/// The CKPT (checkpoint) section describes checkpoints; the routes players must follow to count laps. The racers must follow the path described by each group of points (as determined by CKPH). More than 0xFF (255) entries are possible if the last group begins at index ≤254. This is not recommended because Lakitu will always appear on-screen.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Ckpt {
    cp_left: [f32; 2],
    cp_right: [f32; 2],
    respawn_pos: u8,
    cp_type: u8,
    prev_cp: u8,
    next_cp: u8,
}
impl KmpData for Ckpt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let cp_left = [rdr.read_f32::<BE>()?, rdr.read_f32::<BE>()?];
        let cp_right = [rdr.read_f32::<BE>()?, rdr.read_f32::<BE>()?];
        let respawn_pos = rdr.read_u8()?;
        let cp_type = rdr.read_u8()?;
        let prev_cp = rdr.read_u8()?;
        let next_cp = rdr.read_u8()?;
        Ok(Ckpt {
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
impl KmpSectionName for Ckpt {
    fn section_name() -> String {
        "ckpt".into()
    }
}
impl KmpPathSectionName for Ckpt {
    fn path_section_name() -> String {
        "ckph".into()
    }
}

/// The GOBJ (geo object) section describes objects; things such as item boxes, pipes and also controlled objects such as sound triggers.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Gobj {
    pub object_id: u16,
    /// this is part of the extended presence flags, but the value must be 0 if the object does not use this extension
    padding: u16,
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub route: u16,
    pub settings: [u16; 8],
    pub presence_flags: u16,
}
impl KmpData for Gobj {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let object_id = rdr.read_u16::<BE>()?;
        let padding = rdr.read_u16::<BE>()?;
        let position = rdr.read_vec3()?;
        let rotation = rdr.read_vec3()?;
        let scale = rdr.read_vec3()?;
        let route = rdr.read_u16::<BE>()?;
        let settings = rdr.read_array::<u16, 8>()?;
        let presence_flags = rdr.read_u16::<BE>()?;
        Ok(Gobj {
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
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.rotation)?;
        wtr.write_vec3(self.scale)?;
        wtr.write_u16::<BE>(self.route)?;
        wtr.write_array(self.settings)?;
        wtr.write_u16::<BE>(self.presence_flags)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Gobj {
    fn section_name() -> String {
        "gobj".into()
    }
}

/// Each POTI entry can contain a number of POTI entries/points.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct PotiPoint {
    pub position: Vec3,
    pub setting_1: u16,
    pub setting_2: u16,
}
impl KmpData for PotiPoint {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u16::<BE>()?;
        Ok(PotiPoint {
            position,
            setting_1,
            setting_2,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_vec3(self.position)?;
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u16::<BE>(self.setting_2)?;
        Ok(wtr)
    }
}

/// The POTI (point information) section describes routes; these are routes for many things including cameras and objects.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Poti {
    pub num_points: u16,
    pub setting_1: u8,
    pub setting_2: u8,
    pub routes: Vec<PotiPoint>,
}
impl KmpData for Poti {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let num_points = rdr.read_u16::<BE>()?;
        let setting_1 = rdr.read_u8()?;
        let setting_2 = rdr.read_u8()?;
        let mut routes = Vec::new();
        for _ in 0..num_points {
            routes.push(PotiPoint::read(&mut rdr)?);
        }
        Ok(Poti {
            num_points,
            setting_1,
            setting_2,
            routes,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write + Read + Seek,
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
impl KmpSectionName for Poti {
    fn section_name() -> String {
        "poti".into()
    }
}

/// The AREA (area) section describes areas; used to determine which camera to use, for example. The size is 5000 for both the positive and negative sides of the X and Z-axes, and 10000 for only the positive side of the Y-axis.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Area {
    pub shape: u8,
    pub kind: u8,
    pub came_index: u8,
    pub priority: u8,
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub setting_1: u16,
    pub setting_2: u16,
    pub route: u8,
    pub enpt_id: u8,
}
impl KmpData for Area {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let shape = rdr.read_u8()?;
        if shape > 1 {
            return invalid_data_error!("Area shape greater than 1");
        }
        let kind = rdr.read_u8()?;
        if kind > 10 {
            return invalid_data_error!("Area type greater than 10 (0x0A)");
        }
        let came_index = rdr.read_u8()?;
        let priority = rdr.read_u8()?;
        let position = rdr.read_vec3()?;
        let rotation = rdr.read_vec3()?;
        let scale = rdr.read_vec3()?;
        let setting_1 = rdr.read_u16::<BE>()?;
        let setting_2 = rdr.read_u16::<BE>()?;
        let route = rdr.read_u8()?;
        let enpt_id = rdr.read_u8()?;
        rdr.read_u16::<BE>()?;
        Ok(Area {
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
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.rotation)?;
        wtr.write_vec3(self.scale)?;
        wtr.write_u16::<BE>(self.setting_1)?;
        wtr.write_u16::<BE>(self.setting_2)?;
        wtr.write_u8(self.route)?;
        wtr.write_u8(self.enpt_id)?;
        wtr.write_u16::<BE>(0)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Area {
    fn section_name() -> String {
        "area".into()
    }
}

/// The CAME (camera) section describes cameras; used to determine cameras for starting routes, Time Trial pans, etc.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Came {
    pub kind: u8,
    pub next_index: u8,
    pub shake: u8,
    pub route: u8,
    pub point_velocity: u16,
    pub zoom_velocity: u16,
    pub view_velocity: u16,
    pub start: u8,
    pub movie: u8,
    pub position: Vec3,
    pub rotation: Vec3,
    pub zoom_start: f32,
    pub zoom_end: f32,
    pub view_start: Vec3,
    pub view_end: Vec3,
    pub time: f32,
}
impl KmpData for Came {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let kind = rdr.read_u8()?;
        let next_index = rdr.read_u8()?;
        let shake = rdr.read_u8()?;
        let route = rdr.read_u8()?;
        let point_velocity = rdr.read_u16::<BE>()?;
        let zoom_velocity = rdr.read_u16::<BE>()?;
        let view_velocity = rdr.read_u16::<BE>()?;
        let start = rdr.read_u8()?;
        let movie = rdr.read_u8()?;
        let position = rdr.read_vec3()?;
        let rotation = rdr.read_vec3()?;
        let zoom_start = rdr.read_f32::<BE>()?;
        let zoom_end = rdr.read_f32::<BE>()?;
        let view_start = rdr.read_vec3()?;
        let view_end = rdr.read_vec3()?;
        let time = rdr.read_f32::<BE>()?;
        Ok(Came {
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
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.rotation)?;
        wtr.write_f32::<BE>(self.zoom_start)?;
        wtr.write_f32::<BE>(self.zoom_end)?;
        wtr.write_vec3(self.view_start)?;
        wtr.write_vec3(self.view_end)?;
        wtr.write_f32::<BE>(self.time)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Came {
    fn section_name() -> String {
        "came".into()
    }
}

/// The JGPT (jugem point) section describes "Jugem" points; the respawn positions. The index is relevant for the link of the CKPT section.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Jgpt {
    position: Vec3,
    rotation: Vec3,
    respawn_id: u16,
    extra_data: i16,
}
impl KmpData for Jgpt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let rotation = rdr.read_vec3()?;
        let respawn_id = rdr.read_u16::<BE>()?;
        let extra_data = rdr.read_i16::<BE>()?;
        Ok(Jgpt {
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
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.rotation)?;
        wtr.write_u16::<BE>(self.respawn_id)?;
        wtr.write_i16::<BE>(self.extra_data)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Jgpt {
    fn section_name() -> String {
        "jgpt".into()
    }
}

/// The CNPT (cannon point) section describes cannon points; the cannon target positions.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Cnpt {
    position: Vec3,
    angle: Vec3,
    id: u16,
    shoot_effect: i16,
}
impl KmpData for Cnpt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let angle = rdr.read_vec3()?;
        let id = rdr.read_u16::<BE>()?;
        let shoot_effect = rdr.read_i16::<BE>()?;
        Ok(Cnpt {
            position,
            angle,
            id,
            shoot_effect,
        })
    }
    fn write<T>(&self, mut wtr: T) -> io::Result<T>
    where
        T: Write,
    {
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.angle)?;
        wtr.write_u16::<BE>(self.id)?;
        wtr.write_i16::<BE>(self.shoot_effect)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Cnpt {
    fn section_name() -> String {
        "cnpt".into()
    }
}

/// The MSPT (mission success point) section describes end positions. After battles and tournaments have ended, the players are placed on this point(s).
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Mspt {
    position: Vec3,
    angle: Vec3,
    id: u16,
    unknown: u16,
}
impl KmpData for Mspt {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let position = rdr.read_vec3()?;
        let angle = rdr.read_vec3()?;
        let id = rdr.read_u16::<BE>()?;
        let unknown = rdr.read_u16::<BE>()?;
        Ok(Mspt {
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
        wtr.write_vec3(self.position)?;
        wtr.write_vec3(self.angle)?;
        wtr.write_u16::<BE>(self.id)?;
        wtr.write_u16::<BE>(self.unknown)?;
        Ok(wtr)
    }
}
impl KmpSectionName for Mspt {
    fn section_name() -> String {
        "mspt".into()
    }
}

/// The STGI (stage info) section describes stage information; information about a track.
#[derive(Debug, Serialize, Deserialize, Clone, Reflect)]
pub struct Stgi {
    pub lap_count: u8,
    pub pole_pos: u8,
    pub driver_distance: u8,
    pub lens_flare_flashing: u8,
    pub flare_color: [u8; 4],
    /// Always 0 in Nintendo tracks. This is for the speed modifier cheat code.
    pub speed_mod: f32,
}
impl KmpData for Stgi {
    fn read(mut rdr: impl Read) -> io::Result<Self> {
        let lap_count = rdr.read_u8()?;
        let pole_pos = rdr.read_u8()?;
        let driver_distance = rdr.read_u8()?;
        let lens_flare_flashing = rdr.read_u8()?;
        // first byte of flare color not needed
        rdr.read_u8()?;
        let flare_color = rdr.read_array::<u8, 4>()?;
        // padding
        rdr.read_u8()?;
        let mut speed_mod = [0u8; 4];
        speed_mod[0] = rdr.read_u8()?;
        speed_mod[1] = rdr.read_u8()?;
        let speed_mod = f32::from_be_bytes(speed_mod);

        Ok(Stgi {
            lap_count,
            pole_pos,
            driver_distance,
            lens_flare_flashing,
            flare_color,
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
        wtr.write_u8(0)?;
        wtr.write_array(self.flare_color)?;
        wtr.write_u8(0)?;
        // only write the 2 MSBs of the speed mod
        let bytes = self.speed_mod.to_be_bytes();
        wtr.write_array([bytes[0], bytes[1]])?;
        Ok(wtr)
    }
}
impl KmpSectionName for Stgi {
    fn section_name() -> String {
        "stgi".into()
    }
}
