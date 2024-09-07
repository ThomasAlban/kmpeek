use bevy::prelude::*;
use binrw::{binrw, BinRead, BinWrite};
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
};

/// stores all the data of the KMP file
#[derive(Debug, Serialize, Deserialize, Resource, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct KmpFile {
    pub header: Header,
    pub ktpt: Section<Ktpt>,
    pub enpt: Section<Enpt>,
    pub enph: Section<PathGroup<Enpt>>,
    pub itpt: Section<Itpt>,
    pub itph: Section<PathGroup<Itpt>>,
    pub ckpt: Section<Ckpt>,
    pub ckph: Section<PathGroup<Ckpt>>,
    pub gobj: Section<Gobj>,
    pub poti: Section<Poti>,
    pub area: Section<Area>,
    pub came: Section<Came>,
    pub jgpt: Section<Jgpt>,
    pub cnpt: Section<Cnpt>,
    pub mspt: Section<Mspt>,
    pub stgi: Section<Stgi>,
}

/// The header, which contains general information about the KMP
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
#[brw(magic = b"RKMD", big)]
#[br(assert(num_sections == 15, "number of sections in header was not 15"))]
pub struct Header {
    // currently the file_len field will be incorrect when writing back to the header
    file_len: u32,
    num_sections: u16,
    header_len: u16,
    version_num: u32,
    section_offsets: [u32; 15],
}

/// Each section has a header containing its info (like the name and number of entries)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct SectionHeader {
    pub section_name: [u8; 4],
    pub num_entries: u16,
    /// The POTI section stores the total number of points of all routes here. The CAME section stores different values. For all other sections, the value is 0 (padding).
    pub additional_value: u16,
}

/// A generic type for a section of a KMP - each section contains a header, and a number of entries.
#[derive(Debug, Serialize, Deserialize, Clone, Default, Deref, DerefMut)]
#[binrw]
#[brw(big)]
pub struct Section<T>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'a,
    for<'a> T: BinWrite<Args<'a> = ()> + 'a,
{
    pub section_header: SectionHeader,
    #[br(count = usize::from(section_header.num_entries))]
    #[deref]
    pub entries: Vec<T>,
}

/// The KTPT (kart point) section describes kart points; the starting position for racers.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Ktpt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    #[brw(pad_after = 2)]
    pub player_index: i16,
}

/// The ENPT (enemy point) section describes enemy points; the routes of CPU racers. The CPU racers attempt to follow the path described by each group of points (as determined by ENPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Enpt {
    pub position: [f32; 3],
    pub leniency: f32,
    pub setting_1: u16,
    pub setting_2: u8,
    pub setting_3: u8,
}

/// The PathGroup section describes the structure of ENPH, ITPH, and CKPH groups:
/// * The ENPH (enemy path) section describes enemy point grouping; how the routes of CPU racers link together.
/// * The ITPH (item path) section describes item point grouping; how the item routes link together. When all previous or next group indices are set to 0xFF, the game assumes the order of points as they appear in the ITPT section.
/// * The CKPH (checkpoint path) section describes checkpoint grouping; how the routes of checkpoints link together.
#[derive(Debug, Serialize, Deserialize, Clone, Deref, DerefMut, Default, new)]
#[binrw]
#[brw(big)]
pub struct PathGroup<T: 'static + Default> {
    pub start: u8,
    pub group_length: u8,
    pub prev_group: [u8; 6],
    pub next_group: [u8; 6],
    pub group_link: u16,
    #[serde(skip_serializing)]
    #[deref]
    _p: PhantomData<T>,
}

/// The ITPT (item point) section describes item points; the Red Shell and Bullet Bill routes. The items attempt to follow the path described by each group of points (as determined by ITPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[binrw]
#[brw(big)]
pub struct Itpt {
    pub position: [f32; 3],
    pub bullet_control: f32,
    pub setting_1: u16,
    pub setting_2: u16,
}

/// The CKPT (checkpoint) section describes checkpoints; the routes players must follow to count laps. The racers must follow the path described by each group of points (as determined by CKPH). More than 0xFF (255) entries are possible if the last group begins at index ≤254. This is not recommended because Lakitu will always appear on-screen.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Ckpt {
    pub cp_left: [f32; 2],
    pub cp_right: [f32; 2],
    pub respawn_pos: u8,
    pub cp_type: i8,
    pub prev_cp: u8,
    pub next_cp: u8,
}

/// The GOBJ (geo object) section describes objects; things such as item boxes, pipes and also controlled objects such as sound triggers.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Gobj {
    pub object_id: u16,
    /// * this is part of the extended presence flags, but the value must be 0 if the object does not use this extension
    pub padding: u16,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub route: u16,
    pub settings: [u16; 8],
    pub presence_flags: u16,
}

/// Each POTI entry can contain a number of POTI entries/points.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct PotiPoint {
    pub position: [f32; 3],
    pub setting_1: u16,
    pub setting_2: u16,
}

/// The POTI (point information) section describes routes; these are routes for many things including cameras and objects.
#[derive(Debug, Serialize, Deserialize, Clone, Deref, DerefMut, Default)]
#[binrw]
#[brw(big)]
pub struct Poti {
    pub num_points: u16,
    pub setting_1: u8,
    pub setting_2: u8,
    #[br(count = usize::from(num_points))]
    #[deref]
    pub points: Vec<PotiPoint>,
}

/// The AREA (area) section describes areas; used to determine which camera to use, for example. The size is 5000 for both the positive and negative sides of the X and Z-axes, and 10000 for only the positive side of the Y-axis.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Area {
    pub shape: u8,
    pub kind: u8,
    pub came_index: u8,
    pub priority: u8,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub setting_1: u16,
    pub setting_2: u16,
    pub route: u8,
    #[brw(pad_after = 2)]
    pub enpt_id: u8,
}

/// The CAME (camera) section describes cameras; used to determine cameras for starting routes, Time Trial pans, etc.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
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
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub zoom_start: f32,
    pub zoom_end: f32,
    pub view_start: [f32; 3],
    pub view_end: [f32; 3],
    pub time: f32,
}

/// The JGPT (jugem point) section describes "Jugem" points; the respawn positions. The index is relevant for the link of the CKPT section.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Jgpt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub respawn_id: u16,
    pub extra_data: i16,
}

/// The CNPT (cannon point) section describes cannon points; the cannon target positions.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Cnpt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    #[brw(pad_before = 2)] // this is the id field which is irrelevant as it is just the index
    pub shoot_effect: i16,
}

/// The MSPT (mission success point) section describes end positions. After battles and tournaments have ended, the players are placed on this point(s).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Mspt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    #[brw(pad_before = 2)] // this is the id field which is irrelevant as it is just the index
    pub unknown: u16,
}

/// The STGI (stage info) section describes stage information; information about a track.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Stgi {
    pub lap_count: u8,
    pub pole_pos: u8,
    pub driver_distance: u8,
    pub lens_flare_flashing: u8,
    #[brw(pad_before = 1)]
    pub flare_color: [u8; 4],
    pub padding_1: u16,
    pub padding_2: u8,
    // #[brw(pad_before = 1)]
    // /// * Always 0 in Nintendo tracks. This is for the speed modifier cheat code.
    // pub speed_mod: f32,
}

impl KmpFile {
    const HEADER_LEN: u16 = 0x4c;

    pub fn read<R: Read + Seek>(r: &mut R) -> anyhow::Result<Self> {
        let mut kmp = KmpFile {
            header: Header::read(r)?,
            ..default()
        };

        kmp.read_kmp_section::<Ktpt, _>(r, 0)?;
        kmp.read_kmp_section::<Enpt, _>(r, 1)?;
        kmp.read_kmp_section::<PathGroup<Enpt>, _>(r, 2)?;
        kmp.read_kmp_section::<Itpt, _>(r, 3)?;
        kmp.read_kmp_section::<PathGroup<Itpt>, _>(r, 4)?;
        kmp.read_kmp_section::<Ckpt, _>(r, 5)?;
        kmp.read_kmp_section::<PathGroup<Ckpt>, _>(r, 6)?;
        kmp.read_kmp_section::<Gobj, _>(r, 7)?;
        kmp.read_kmp_section::<Poti, _>(r, 8)?;
        kmp.read_kmp_section::<Area, _>(r, 9)?;
        kmp.read_kmp_section::<Came, _>(r, 10)?;
        kmp.read_kmp_section::<Jgpt, _>(r, 11)?;
        kmp.read_kmp_section::<Cnpt, _>(r, 12)?;
        kmp.read_kmp_section::<Mspt, _>(r, 13)?;
        kmp.read_kmp_section::<Stgi, _>(r, 14)?;

        Ok(kmp)
    }

    pub fn write<W: Write + Seek>(mut self, w: &mut W) -> anyhow::Result<()> {
        // skip the header for now
        w.seek(SeekFrom::Start(Self::HEADER_LEN as u64))?;

        self.write_kmp_section::<Ktpt, _>(w, 0)?;
        self.write_kmp_section::<Enpt, _>(w, 1)?;
        self.write_kmp_section::<PathGroup<Enpt>, _>(w, 2)?;
        self.write_kmp_section::<Itpt, _>(w, 3)?;
        self.write_kmp_section::<PathGroup<Itpt>, _>(w, 4)?;
        self.write_kmp_section::<Ckpt, _>(w, 5)?;
        self.write_kmp_section::<PathGroup<Ckpt>, _>(w, 6)?;
        self.write_kmp_section::<Gobj, _>(w, 7)?;
        self.write_kmp_section::<Poti, _>(w, 8)?;
        self.write_kmp_section::<Area, _>(w, 9)?;
        self.write_kmp_section::<Came, _>(w, 10)?;
        self.write_kmp_section::<Jgpt, _>(w, 11)?;
        self.write_kmp_section::<Cnpt, _>(w, 12)?;
        self.write_kmp_section::<Mspt, _>(w, 13)?;
        self.write_kmp_section::<Stgi, _>(w, 14)?;

        // todo: go back to the start and write the header
        w.seek(SeekFrom::Start(0))?;
        self.header.write(w)?;

        Ok(())
    }

    fn read_kmp_section<T, R: Read + Seek>(&mut self, r: &mut R, i: usize) -> anyhow::Result<()>
    where
        for<'a> T: BinRead<Args<'a> = ()> + 'a,
        T: KmpGetSection,
    {
        r.seek(SeekFrom::Start(
            self.header.section_offsets[i] as u64 + Self::HEADER_LEN as u64,
        ))?;
        *T::get_section_mut(self) = Section::<T>::read(r)?;
        Ok(())
    }
    fn write_kmp_section<T, W: Write + Seek>(&mut self, w: &mut W, i: usize) -> anyhow::Result<()>
    where
        for<'a> T: BinWrite<Args<'a> = ()> + 'a,
        T: KmpGetSection,
    {
        self.header.section_offsets[i] = w.stream_position()? as u32 - Self::HEADER_LEN as u32;
        T::get_section(self).write(w)?;
        Ok(())
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            file_len: 0,
            num_sections: 15,
            header_len: 0x4c,
            version_num: 0x9d8,
            section_offsets: [0; 15],
        }
    }
}

impl<T> Section<T>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'a,
    for<'a> T: BinWrite<Args<'a> = ()> + 'a,
    T: KmpSectionName,
{
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            section_header: SectionHeader {
                section_name: T::SECTION_NAME,
                num_entries: entries.len() as u16,
                additional_value: 0,
            },
            entries,
        }
    }
}

pub trait KmpSectionName {
    const SECTION_NAME: [u8; 4];
}
macro_rules! impl_kmp_sect_name {
    ($ty:ty, $name:expr) => {
        impl KmpSectionName for $ty {
            const SECTION_NAME: [u8; 4] = *$name;
        }
    };
}

impl_kmp_sect_name!(Ktpt, b"KTPT");
impl_kmp_sect_name!(Enpt, b"ENPT");
impl_kmp_sect_name!(PathGroup<Enpt>, b"ENPH");
impl_kmp_sect_name!(Itpt, b"ITPT");
impl_kmp_sect_name!(PathGroup<Itpt>, b"ITPH");
impl_kmp_sect_name!(Ckpt, b"CKPT");
impl_kmp_sect_name!(PathGroup<Ckpt>, b"CKPH");
impl_kmp_sect_name!(Gobj, b"GOBJ");
impl_kmp_sect_name!(Poti, b"POTI");
impl_kmp_sect_name!(PotiPoint, b"POTI");
impl_kmp_sect_name!(Area, b"AREA");
impl_kmp_sect_name!(Came, b"CAME");
impl_kmp_sect_name!(Jgpt, b"JGPT");
impl_kmp_sect_name!(Cnpt, b"CNPT");
impl_kmp_sect_name!(Mspt, b"MSPT");
impl_kmp_sect_name!(Stgi, b"STGI");

pub trait KmpGetSection
where
    for<'a> Self: BinRead<Args<'a> = ()> + 'a,
    for<'a> Self: BinWrite<Args<'a> = ()> + 'a,
{
    fn get_section(kmp: &KmpFile) -> &Section<Self>;
    fn get_section_mut(kmp: &mut KmpFile) -> &mut Section<Self>;
}
pub trait KmpGetPathSection
where
    Self: Sized + Default,
{
    fn get_path_section(kmp: &KmpFile) -> &Section<PathGroup<Self>>;
    fn get_path_section_mut(kmp: &mut KmpFile) -> &mut Section<PathGroup<Self>>;
}
macro_rules! impl_kmp_get_section {
    ($kmp_section:ty, $sect:ident) => {
        impl KmpGetSection for $kmp_section {
            fn get_section(kmp: &KmpFile) -> &Section<Self> {
                &kmp.$sect
            }
            fn get_section_mut(kmp: &mut KmpFile) -> &mut Section<Self> {
                &mut kmp.$sect
            }
        }
    };
}
macro_rules! impl_kmp_path_section {
    ($kmp_sect:ty, $sect:ident) => {
        impl KmpGetPathSection for $kmp_sect {
            fn get_path_section(kmp: &KmpFile) -> &Section<PathGroup<$kmp_sect>> {
                &kmp.$sect
            }
            fn get_path_section_mut(kmp: &mut KmpFile) -> &mut Section<PathGroup<$kmp_sect>> {
                &mut kmp.$sect
            }
        }
    };
}
impl_kmp_get_section!(Ktpt, ktpt);
impl_kmp_get_section!(Enpt, enpt);
impl_kmp_get_section!(Itpt, itpt);
impl_kmp_get_section!(Ckpt, ckpt);
impl_kmp_get_section!(Area, area);
impl_kmp_get_section!(Gobj, gobj);
impl_kmp_get_section!(Poti, poti);
impl_kmp_get_section!(Came, came);
impl_kmp_get_section!(Jgpt, jgpt);
impl_kmp_get_section!(Cnpt, cnpt);
impl_kmp_get_section!(Mspt, mspt);
impl_kmp_get_section!(Stgi, stgi);

impl_kmp_get_section!(PathGroup<Enpt>, enph);
impl_kmp_get_section!(PathGroup<Itpt>, itph);
impl_kmp_get_section!(PathGroup<Ckpt>, ckph);

impl_kmp_path_section!(Enpt, enph);
impl_kmp_path_section!(Itpt, itph);
impl_kmp_path_section!(Ckpt, ckph);

pub trait KmpPositionPoint {
    fn get_position(&self) -> [f32; 3];
}
macro_rules! impl_kmp_position_point {
    ($kmp_section:ty) => {
        impl KmpPositionPoint for $kmp_section {
            fn get_position(&self) -> [f32; 3] {
                self.position
            }
        }
    };
}
impl_kmp_position_point!(Ktpt);
impl_kmp_position_point!(Enpt);
impl_kmp_position_point!(Itpt);
impl_kmp_position_point!(Area);
impl_kmp_position_point!(Gobj);
impl_kmp_position_point!(Came);
impl_kmp_position_point!(Jgpt);
impl_kmp_position_point!(Cnpt);
impl_kmp_position_point!(Mspt);

pub trait KmpRotationPoint {
    fn get_rotation(&self) -> [f32; 3];
}
macro_rules! impl_kmp_rotation_point {
    ($kmp_section:ty) => {
        impl KmpRotationPoint for $kmp_section {
            fn get_rotation(&self) -> [f32; 3] {
                self.rotation
            }
        }
    };
}
impl_kmp_rotation_point!(Ktpt);
impl_kmp_rotation_point!(Area);
impl_kmp_rotation_point!(Gobj);
impl_kmp_rotation_point!(Came);
impl_kmp_rotation_point!(Jgpt);
impl_kmp_rotation_point!(Cnpt);
impl_kmp_rotation_point!(Mspt);

pub trait MaybeRouteId {
    fn get_route_id(&self) -> Option<u8>;
}
macro_rules! impl_no_route_id {
    ($kmp_section:ty) => {
        impl MaybeRouteId for $kmp_section {
            fn get_route_id(&self) -> Option<u8> {
                None
            }
        }
    };
}

impl MaybeRouteId for Gobj {
    fn get_route_id(&self) -> Option<u8> {
        (self.route != 0xffff).then_some(self.route as u8)
    }
}
impl MaybeRouteId for Area {
    fn get_route_id(&self) -> Option<u8> {
        (self.kind == 3).then_some(self.route)
    }
}
impl MaybeRouteId for Came {
    fn get_route_id(&self) -> Option<u8> {
        (self.route != 0xff).then_some(self.route)
    }
}
impl_no_route_id!(Ktpt);
impl_no_route_id!(Enpt);
impl_no_route_id!(Itpt);
impl_no_route_id!(Ckpt);
impl_no_route_id!(PotiPoint);
impl_no_route_id!(Jgpt);
impl_no_route_id!(Cnpt);
impl_no_route_id!(Mspt);

#[test]
fn test_full_rewrite() {
    read_write_kmp_test("test_files/desert_course/course.kmp");
    read_write_kmp_test("test_files/boardcross_course/course.kmp");
    read_write_kmp_test("test_files/shopping_course/course.kmp");
}

#[allow(dead_code)]
fn read_write_kmp_test(path: &str) {
    let mut input_file = File::open(path).unwrap();

    let input_length = input_file.seek(SeekFrom::End(0)).unwrap() as usize;
    input_file.seek(SeekFrom::Start(0)).unwrap();

    let mut in_buf = vec![0u8; input_length];

    input_file.read_exact(&mut in_buf).unwrap();
    drop(input_file);

    let mut in_cursor = Cursor::new(&mut in_buf);

    let kmp = KmpFile::read(&mut in_cursor).unwrap();

    let mut out_buf: Vec<u8> = Vec::new();
    let mut out_cursor = Cursor::new(&mut out_buf);

    kmp.write(&mut out_cursor).unwrap();

    for (i, (in_byte, out_byte)) in in_buf.iter().zip(out_buf.iter()).enumerate() {
        assert_eq!(
            in_byte, out_byte,
            "Mismatching byte at {}, in: {:02X}, out: {:02X}",
            i, in_byte, out_byte
        );
    }
}
