use bevy::prelude::*;
use binrw::{binrw, BinRead, BinWrite};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
};

/// stores all the data of the KMP file
#[derive(Debug, Serialize, Deserialize, Resource, Clone)]
#[binrw]
#[brw(big)]
pub struct KmpFile {
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

/// The header, which contains general information about the KMP
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
#[brw(magic = b"RKMD")]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
struct SectionHeader {
    pub section_name: [u8; 4],
    pub num_entries: u16,
    /// The POTI section stores the total number of points of all routes here. The CAME section stores different values. For all other sections, the value is 0 (padding).
    pub additional_value: u16,
}

/// A generic type for a section of a KMP - each section contains a header, and a number of entries.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Section<T>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'a,
    for<'a> T: BinWrite<Args<'a> = ()> + 'a,
{
    section_header: SectionHeader,
    #[br(count = usize::from(section_header.num_entries))]
    pub entries: Vec<T>,
}

/// The KTPT (kart point) section describes kart points; the starting position for racers.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Ktpt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    #[brw(pad_after = 2)]
    pub player_index: i16,
}

/// The ENPT (enemy point) section describes enemy points; the routes of CPU racers. The CPU racers attempt to follow the path described by each group of points (as determined by ENPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct PathGroup {
    pub start: u8,
    pub group_length: u8,
    pub prev_group: [u8; 6],
    pub next_group: [u8; 6],
    pub group_link: u16,
}

/// The ITPT (item point) section describes item points; the Red Shell and Bullet Bill routes. The items attempt to follow the path described by each group of points (as determined by ITPH). More than 0xFF (255) entries will force a console freeze while loading the track.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[binrw]
pub struct Itpt {
    pub position: [f32; 3],
    pub bullet_control: f32,
    pub setting_1: u16,
    pub setting_2: u16,
}

/// The CKPT (checkpoint) section describes checkpoints; the routes players must follow to count laps. The racers must follow the path described by each group of points (as determined by CKPH). More than 0xFF (255) entries are possible if the last group begins at index ≤254. This is not recommended because Lakitu will always appear on-screen.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Ckpt {
    pub cp_left: [f32; 2],
    pub cp_right: [f32; 2],
    pub respawn_pos: u8,
    pub cp_type: i8,
    pub prev_cp: u8,
    pub next_cp: u8,
}

/// The GOBJ (geo object) section describes objects; things such as item boxes, pipes and also controlled objects such as sound triggers.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Gobj {
    pub object_id: u16,
    /// * this is part of the extended presence flags, but the value must be 0 if the object does not use this extension
    padding: u16,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub route: u16,
    pub settings: [u16; 8],
    pub presence_flags: u16,
}

/// Each POTI entry can contain a number of POTI entries/points.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct PotiPoint {
    pub position: [f32; 3],
    pub setting_1: u16,
    pub setting_2: u16,
}

/// The POTI (point information) section describes routes; these are routes for many things including cameras and objects.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Poti {
    pub num_points: u16,
    pub setting_1: u8,
    pub setting_2: u8,
    #[br(count = usize::from(num_points))]
    pub routes: Vec<PotiPoint>,
}

/// The AREA (area) section describes areas; used to determine which camera to use, for example. The size is 5000 for both the positive and negative sides of the X and Z-axes, and 10000 for only the positive side of the Y-axis.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Jgpt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub respawn_id: u16,
    pub extra_data: i16,
}

/// The CNPT (cannon point) section describes cannon points; the cannon target positions.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
#[brw(big)]
pub struct Cnpt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    #[brw(pad_before = 2)] // this is the id field which is irrelevant as it is just the index
    pub shoot_effect: i16,
}

/// The MSPT (mission success point) section describes end positions. After battles and tournaments have ended, the players are placed on this point(s).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Mspt {
    position: [f32; 3],
    rotation: [f32; 3],
    #[brw(pad_before = 2)] // this is the id field which is irrelevant as it is just the index
    unknown: u16,
}

/// The STGI (stage info) section describes stage information; information about a track.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[binrw]
pub struct Stgi {
    pub lap_count: u8,
    pub pole_pos: u8,
    pub driver_distance: u8,
    pub lens_flare_flashing: u8,
    pub flare_color: [u8; 4],
    pub padding_1: u16,
    pub padding_2: u8,
    // #[brw(pad_before = 1)]
    // /// * Always 0 in Nintendo tracks. This is for the speed modifier cheat code.
    // pub speed_mod: f32,
}

pub trait KmpGetSection
where
    for<'a> Self: BinRead<Args<'a> = ()> + 'a,
    for<'a> Self: BinWrite<Args<'a> = ()> + 'a,
{
    fn get_section(kmp: &KmpFile) -> &Section<Self>;
    fn get_section_mut(kmp: &mut KmpFile) -> &mut Section<Self>;
}
pub trait KmpGetPathSection {
    fn get_path_section(kmp: &KmpFile) -> &Section<PathGroup>;
    fn get_path_section_mut(kmp: &mut KmpFile) -> &mut Section<PathGroup>;
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
    ($kmp_section:ty, $sect:ident) => {
        impl KmpGetPathSection for $kmp_section {
            fn get_path_section(kmp: &KmpFile) -> &Section<PathGroup> {
                &kmp.$sect
            }
            fn get_path_section_mut(kmp: &mut KmpFile) -> &mut Section<PathGroup> {
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
