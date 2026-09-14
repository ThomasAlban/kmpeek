use bevy::prelude::*;
use binrw::{binrw, BinRead, BinWrite};
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
};

/// stores all the data of the KMP file
#[derive(Debug, Serialize, Deserialize, Resource, Clone, Default)]
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
    pub player_index: i16,
    /// Retain unknown bytes rather than silently zeroing them on raw write.
    #[serde(default)]
    pub padding: u16,
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
    #[serde(skip)]
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
    pub enpt_id: u8,
    /// Unknown trailing word, preserved by the raw serializer.
    #[serde(default)]
    pub padding: u16,
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
    pub duration: f32,
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
    // This is a stored point ID, not padding. Canonical saves use section order.
    #[serde(default)]
    pub id: u16,
    pub shoot_effect: i16,
}

/// The MSPT (mission success point) section describes end positions. After battles and tournaments have ended, the players are placed on this point(s).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[binrw]
#[brw(big)]
pub struct Mspt {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    // This is a stored point ID, not padding. Canonical saves use section order.
    #[serde(default)]
    pub id: u16,
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
    pub flare_color: [u8; 4],
    /// Raw bytes at offsets 8..10 (including unknown/padding data).
    pub padding_1: u16,
    /// Raw bytes at offsets 10..12 (including the speed modifier extension).
    pub padding_2: u16,
}

impl Stgi {
    /// LS-Mod stores the high 16 bits of an IEEE-754 f32, NOT a half float.
    /// Keep encoded zero distinct from explicit 1.0; both mean normal speed.
    /// https://mkwiiki.org/wiki/Lap_%26_Speed_Modifier#How_it_works_(STGI)
    pub fn speed_mod(&self) -> f32 {
        f32::from_bits(u32::from(self.padding_2) << 16)
    }

    pub fn encode_speed_mod(value: f32) -> u16 {
        // The extension has no low mantissa word; saving truncates that precision.
        (value.to_bits() >> 16) as u16
    }
}

impl KmpFile {
    const HEADER_LEN: u16 = 0x4c;

    pub fn read<R: Read + Seek>(r: &mut R) -> anyhow::Result<Self> {
        r.seek(SeekFrom::Start(0))?;
        let mut kmp = KmpFile {
            header: Header::read(r)?,
            ..default()
        };
        let header = &kmp.header;
        anyhow::ensure!(header.header_len >= Self::HEADER_LEN, "KMP header is too short");
        let actual_len = r.seek(SeekFrom::End(0))?;
        anyhow::ensure!(
            u64::from(header.file_len) <= actual_len && header.file_len >= u32::from(header.header_len),
            "KMP declared file length is out of bounds"
        );
        let mut starts = Vec::with_capacity(15);
        for &offset in &header.section_offsets {
            let start = u64::from(header.header_len) + u64::from(offset);
            anyhow::ensure!(
                start + 8 <= u64::from(header.file_len),
                "KMP section offset is out of bounds"
            );
            starts.push(start);
        }
        starts.sort_unstable();
        anyhow::ensure!(
            starts.windows(2).all(|pair| pair[0] + 8 <= pair[1]),
            "KMP section headers overlap"
        );

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

        let total_points = kmp.poti.entries.iter().try_fold(0usize, |total, route| {
            total
                .checked_add(route.points.len())
                .ok_or_else(|| anyhow::anyhow!("POTI point total overflow"))
        })?;
        anyhow::ensure!(
            total_points == usize::from(kmp.poti.section_header.additional_value),
            "POTI total point count mismatch"
        );
        Ok(kmp)
    }

    /// Write a canonical, packed KMP at stream offset zero. Validation/encoding
    /// completes before the destination is touched. The caller must truncate an
    /// existing destination: `Write + Seek` cannot remove stale trailing bytes.
    pub fn write<W: Write + Seek>(mut self, destination: &mut W) -> anyhow::Result<()> {
        self.header.num_sections = 15;
        self.header.header_len = Self::HEADER_LEN;
        let mut total_points = 0usize;
        for route in &mut self.poti.entries {
            route.num_points = u16::try_from(route.points.len())?;
            total_points = total_points
                .checked_add(route.points.len())
                .ok_or_else(|| anyhow::anyhow!("POTI point total overflow"))?;
        }
        self.poti.section_header.additional_value = u16::try_from(total_points)?;
        let mut buffer = Cursor::new(vec![0; usize::from(Self::HEADER_LEN)]);
        let w = &mut buffer;
        w.seek(SeekFrom::Start(u64::from(Self::HEADER_LEN)))?;

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

        self.header.file_len = u32::try_from(w.stream_position()?)?;
        w.seek(SeekFrom::Start(0))?;
        self.header.write(w)?;
        destination.seek(SeekFrom::Start(0))?;
        destination.write_all(buffer.get_ref())?;
        Ok(())
    }

    fn read_kmp_section<T, R: Read + Seek>(&mut self, r: &mut R, i: usize) -> anyhow::Result<()>
    where
        for<'a> T: BinRead<Args<'a> = ()> + 'a,
        T: KmpGetSection + KmpSectionName,
    {
        let offset = self.header.section_offsets[i];
        let start = u64::from(self.header.header_len) + u64::from(offset);
        // Sections may be physically reordered. The nearest higher offset, not
        // the next table entry, bounds this section's records.
        let end = self
            .header
            .section_offsets
            .iter()
            .copied()
            .filter(|&other| other > offset)
            .min()
            .map(|other| u64::from(self.header.header_len) + u64::from(other))
            .unwrap_or(u64::from(self.header.file_len));
        r.seek(SeekFrom::Start(start))?;
        let mut bounded = SectionReader { inner: r, start, end };
        let section_header = SectionHeader::read(&mut bounded)?;
        anyhow::ensure!(
            section_header.section_name == T::SECTION_NAME,
            "unexpected KMP section name at index {i}"
        );
        let mut entries = Vec::new();
        for _ in 0..section_header.num_entries {
            entries.push(T::read_options(&mut bounded, binrw::Endian::Big, ())?);
        }
        *T::get_section_mut(self) = Section {
            section_header,
            entries,
        };
        Ok(())
    }
    fn write_kmp_section<T, W: Write + Seek>(&mut self, w: &mut W, i: usize) -> anyhow::Result<()>
    where
        for<'a> T: BinWrite<Args<'a> = ()> + 'a,
        T: KmpGetSection + KmpSectionName,
    {
        self.header.section_offsets[i] = checked_section_offset(w.stream_position()?)?;
        let section = T::get_section_mut(self);
        section.section_header.section_name = T::SECTION_NAME;
        section.section_header.num_entries = u16::try_from(section.entries.len())?;
        section.write(w)?;
        // Check the end as well as the start of the last section.
        u32::try_from(w.stream_position()?)?;
        Ok(())
    }
}

fn checked_section_offset(position: u64) -> anyhow::Result<u32> {
    // Both absolute file length and relative offsets must fit in the format.
    let position = u32::try_from(position)?;
    position
        .checked_sub(u32::from(KmpFile::HEADER_LEN))
        .ok_or_else(|| anyhow::anyhow!("section begins before the KMP header"))
}

/// A seekable view that cannot read or seek outside one declared section.
struct SectionReader<'a, R> {
    inner: &'a mut R,
    start: u64,
    end: u64,
}

impl<R: Read + Seek> Read for SectionReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let position = self.inner.stream_position()?;
        let remaining = self.end.saturating_sub(position);
        let len = buf.len().min(usize::try_from(remaining).unwrap_or(usize::MAX));
        self.inner.read(&mut buf[..len])
    }
}

impl<R: Seek> Seek for SectionReader<'_, R> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let position = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::Current(value) => i128::from(self.inner.stream_position()?) + i128::from(value),
            SeekFrom::End(value) => i128::from(self.end) + i128::from(value),
        };
        if position < i128::from(self.start) || position > i128::from(self.end) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek outside KMP section",
            ));
        }
        self.inner.seek(SeekFrom::Start(position as u64))
    }
}

// Keep trait-based callers on the same validated path as the inherent API.
impl BinRead for KmpFile {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(reader: &mut R, _: binrw::Endian, _: ()) -> binrw::BinResult<Self> {
        KmpFile::read(reader)
            .map_err(|error| binrw::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())))
    }
}

impl BinWrite for KmpFile {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(&self, writer: &mut W, _: binrw::Endian, _: ()) -> binrw::BinResult<()> {
        KmpFile::write(self.clone(), writer)
            .map_err(|error| binrw::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())))
    }
}

impl binrw::meta::ReadEndian for KmpFile {
    const ENDIAN: binrw::meta::EndianKind = binrw::meta::EndianKind::Endian(binrw::Endian::Big);
}

impl binrw::meta::WriteEndian for KmpFile {
    const ENDIAN: binrw::meta::EndianKind = binrw::meta::EndianKind::Endian(binrw::Endian::Big);
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
    /// Construct a section. Panics if the entry count cannot fit in a KMP;
    /// use `try_new` when accepting untrusted or unbounded input.
    pub fn new(entries: Vec<T>) -> Self {
        Self::try_new(entries).expect("KMP section entry count exceeds u16")
    }

    pub fn try_new(entries: Vec<T>) -> anyhow::Result<Self> {
        Ok(Self {
            section_header: SectionHeader {
                section_name: T::SECTION_NAME,
                num_entries: u16::try_from(entries.len())?,
                additional_value: 0,
            },
            entries,
        })
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
    fn get_route_id(&self) -> Option<u16>;
}
macro_rules! impl_no_route_id {
    ($kmp_section:ty) => {
        impl MaybeRouteId for $kmp_section {
            fn get_route_id(&self) -> Option<u16> {
                None
            }
        }
    };
}

impl MaybeRouteId for Gobj {
    fn get_route_id(&self) -> Option<u16> {
        (self.route != 0xffff).then_some(self.route)
    }
}
impl MaybeRouteId for Area {
    fn get_route_id(&self) -> Option<u16> {
        (self.kind == 3 && self.route != 0xff).then_some(u16::from(self.route))
    }
}
impl MaybeRouteId for Came {
    fn get_route_id(&self) -> Option<u16> {
        (self.route != 0xff).then_some(u16::from(self.route))
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

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [[u8; 4]; 15] = [
        *b"KTPT", *b"ENPT", *b"ENPH", *b"ITPT", *b"ITPH", *b"CKPT", *b"CKPH", *b"GOBJ", *b"POTI", *b"AREA", *b"CAME",
        *b"JGPT", *b"CNPT", *b"MSPT", *b"STGI",
    ];
    const SIZES: [usize; 15] = [28, 20, 16, 20, 16, 20, 16, 60, 56, 48, 72, 28, 28, 28, 12];

    fn u16_at(bytes: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap())
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn section_start(bytes: &[u8], index: usize) -> usize {
        usize::from(u16_at(bytes, 10)) + u32_at(bytes, 16 + index * 4) as usize
    }

    fn encode(kmp: KmpFile) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        kmp.write(&mut output).unwrap();
        assert_eq!(output.position() as usize, output.get_ref().len());
        output.into_inner()
    }

    // Independently assemble all fifteen sections, using nonzero raw field
    // bytes (finite floats) rather than relying on the writer under test.
    fn synthetic_file() -> Vec<u8> {
        let mut bytes = vec![0; 0x4c];
        bytes[..4].copy_from_slice(b"RKMD");
        bytes[8..10].copy_from_slice(&15u16.to_be_bytes());
        bytes[10..12].copy_from_slice(&0x4cu16.to_be_bytes());
        bytes[12..16].copy_from_slice(&0x9d8u32.to_be_bytes());
        for i in 0..15 {
            let offset = (bytes.len() - 0x4c) as u32;
            bytes[16 + i * 4..20 + i * 4].copy_from_slice(&offset.to_be_bytes());
            bytes.extend_from_slice(&NAMES[i]);
            bytes.extend_from_slice(&(if i == 8 { 2u16 } else { 1 }).to_be_bytes());
            bytes.extend_from_slice(&(if i == 8 { 3u16 } else { 0x1234 }).to_be_bytes());
            let mut payload: Vec<u8> = (0..SIZES[i]).map(|n| ((n + i) % 0x70 + 1) as u8).collect();
            if i == 8 {
                // Keep patterned nonzero padding/IDs: raw writes must retain them.
                payload[0..2].copy_from_slice(&2u16.to_be_bytes());
                payload[36..38].copy_from_slice(&1u16.to_be_bytes());
            }
            // match i {
            //     // Keep patterned nonzero padding/IDs: raw writes must retain them.
            //     8 => {
            //         payload[0..2].copy_from_slice(&2u16.to_be_bytes());
            //         payload[36..38].copy_from_slice(&1u16.to_be_bytes());
            //     }
            //     _ => {}
            // }
            bytes.extend_from_slice(&payload);
        }
        let len = bytes.len() as u32;
        bytes[4..8].copy_from_slice(&len.to_be_bytes());
        bytes
    }

    #[test]
    fn full_synthetic_roundtrip_preserves_all_supported_fields() {
        let bytes = synthetic_file();
        let kmp = KmpFile::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(kmp.poti.len(), 2);
        assert_eq!(kmp.poti[0].len(), 2);
        assert_eq!(kmp.poti[1].len(), 1);
        assert_eq!(encode(kmp), bytes);
    }

    #[test]
    fn raw_padding_and_point_ids_have_exact_offsets() {
        // Test the actual binary layout, including words formerly skipped by binrw.
        let bytes = synthetic_file();
        let kmp = KmpFile::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(kmp.ktpt[0].padding, u16_at(&bytes, section_start(&bytes, 0) + 8 + 26));
        assert_eq!(kmp.area[0].padding, u16_at(&bytes, section_start(&bytes, 9) + 8 + 46));
        assert_eq!(kmp.cnpt[0].id, u16_at(&bytes, section_start(&bytes, 12) + 8 + 24));
        assert_eq!(kmp.mspt[0].id, u16_at(&bytes, section_start(&bytes, 13) + 8 + 24));
        assert_ne!(kmp.ktpt[0].padding, 0);
        assert_ne!(kmp.area[0].padding, 0);
        assert_ne!(kmp.cnpt[0].id, 0);
        assert_ne!(kmp.mspt[0].id, 0);
        assert_eq!(encode(kmp), bytes);
    }

    #[test]
    fn speed_extension_preserves_every_raw_word() {
        // Includes zeros, subnormals, infinities and NaN payloads: reading raw
        // extension bits must not normalize them or treat them as IEEE half floats.
        for padding_2 in 0..=u16::MAX {
            let stgi = Stgi { padding_2, ..default() };
            assert_eq!(Stgi::encode_speed_mod(stgi.speed_mod()), padding_2);
        }
    }

    #[test]
    fn writer_rebuilds_header_names_counts_and_poti_totals() {
        let mut kmp = KmpFile::read(&mut Cursor::new(synthetic_file())).unwrap();
        kmp.header.file_len = 1;
        kmp.header.header_len = 1;
        kmp.header.num_sections = 1;
        kmp.header.section_offsets.fill(u32::MAX);
        kmp.header.version_num = 0x12345678;
        macro_rules! stale {
            ($($field:ident),*) => {$(
                kmp.$field.section_header.section_name = *b"BAD!";
                kmp.$field.section_header.num_entries = 65535;
            )*};
        }
        stale!(ktpt, enpt, enph, itpt, itph, ckpt, ckph, gobj, poti, area, came, jgpt, cnpt, mspt, stgi);
        kmp.poti.section_header.additional_value = 0;
        for route in &mut kmp.poti.entries {
            route.num_points = 65535;
        }
        let bytes = encode(kmp);
        assert_eq!(u32_at(&bytes, 4) as usize, bytes.len());
        assert_eq!(u16_at(&bytes, 8), 15);
        assert_eq!(u16_at(&bytes, 10), 0x4c);
        assert_eq!(u32_at(&bytes, 12), 0x12345678);
        let mut expected_start = 0x4c;
        for i in 0..15 {
            let start = section_start(&bytes, i);
            assert_eq!(start, expected_start);
            assert_eq!(&bytes[start..start + 4], &NAMES[i]);
            assert_eq!(u16_at(&bytes, start + 4), if i == 8 { 2 } else { 1 });
            assert_eq!(u16_at(&bytes, start + 6), if i == 8 { 3 } else { 0x1234 });
            expected_start += 8 + SIZES[i];
        }
        assert_eq!(expected_start, bytes.len());
        let poti = section_start(&bytes, 8) + 8;
        assert_eq!(u16_at(&bytes, poti), 2);
        assert_eq!(u16_at(&bytes, poti + 36), 1);
        KmpFile::read(&mut Cursor::new(bytes)).unwrap();
    }

    #[test]
    fn stgi_color_and_trailing_raw_bytes_have_exact_layout() {
        let bytes = [3, 1, 2, 1, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        let stgi = Stgi::read(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(stgi.flare_color, [0x12, 0x34, 0x56, 0x78]);
        assert_eq!(stgi.padding_1, 0x9abc);
        assert_eq!(stgi.padding_2, 0xdef0);
        let mut output = Cursor::new(Vec::new());
        stgi.write(&mut output).unwrap();
        assert_eq!(output.into_inner(), bytes);
    }

    #[test]
    fn reads_declared_header_length_and_reordered_sections() {
        let original = synthetic_file();
        let mut extended = original.clone();
        extended.splice(0x4c..0x4c, [0xab; 16]);
        extended[10..12].copy_from_slice(&0x5cu16.to_be_bytes());
        let len = extended.len() as u32;
        extended[4..8].copy_from_slice(&len.to_be_bytes());
        assert_eq!(encode(KmpFile::read(&mut Cursor::new(extended)).unwrap()), original);

        let mut reordered = original[..0x4c].to_vec();
        for i in (0..15).rev() {
            let offset = (reordered.len() - 0x4c) as u32;
            reordered[16 + i * 4..20 + i * 4].copy_from_slice(&offset.to_be_bytes());
            let start = section_start(&original, i);
            reordered.extend_from_slice(&original[start..start + 8 + SIZES[i]]);
        }
        assert_eq!(encode(KmpFile::read(&mut Cursor::new(reordered)).unwrap()), original);
    }

    #[test]
    fn rejects_invalid_headers_names_offsets_counts_and_truncation() {
        let original = synthetic_file();
        let reject = |bytes: Vec<u8>| {
            assert!(KmpFile::read(&mut Cursor::new(bytes)).is_err());
        };
        for (offset, replacement) in [
            (0, b"FAIL".to_vec()),
            (8, 14u16.to_be_bytes().to_vec()),
            (10, 0x4bu16.to_be_bytes().to_vec()),
            (4, u32::MAX.to_be_bytes().to_vec()),
            (4, 0x4cu32.to_be_bytes().to_vec()),
            (16, u32::MAX.to_be_bytes().to_vec()),
            (20, 0u32.to_be_bytes().to_vec()),
            (20, 4u32.to_be_bytes().to_vec()),
            (0x4c, b"ENPT".to_vec()),
            (0x50, 2u16.to_be_bytes().to_vec()),
            (section_start(&original, 8) + 6, 4u16.to_be_bytes().to_vec()),
            (section_start(&original, 8) + 8, u16::MAX.to_be_bytes().to_vec()),
        ] {
            let mut bytes = original.clone();
            bytes[offset..offset + replacement.len()].copy_from_slice(&replacement);
            reject(bytes);
        }
        for len in 0..original.len() {
            reject(original[..len].to_vec());
        }
        // Even when the declared file length is shortened along with the file,
        // an entry must not be read outside the final section.
        let mut shortened = original[..original.len() - 1].to_vec();
        let len = shortened.len() as u32;
        shortened[4..8].copy_from_slice(&len.to_be_bytes());
        reject(shortened);
    }

    #[test]
    fn rejects_count_overflows_without_touching_destination() {
        fn reject(kmp: KmpFile) {
            let mut destination = Cursor::new(vec![0xaa; 100]);
            destination.set_position(17);
            assert!(kmp.write(&mut destination).is_err());
            assert_eq!(destination.position(), 17);
            assert_eq!(destination.into_inner(), vec![0xaa; 100]);
        }
        let mut kmp = KmpFile::default();
        kmp.ktpt.entries = vec![Ktpt::default(); 65536];
        reject(kmp);
        let mut kmp = KmpFile::default();
        kmp.poti.entries = vec![Poti::default(); 65536];
        reject(kmp);
        let mut kmp = KmpFile::default();
        kmp.poti.entries.push(Poti {
            points: vec![PotiPoint::default(); 65536],
            ..default()
        });
        reject(kmp);
        let mut kmp = KmpFile::default();
        kmp.poti.entries = vec![
            Poti {
                points: vec![PotiPoint::default(); 32768],
                ..default()
            };
            2
        ];
        reject(kmp);
        assert!(Section::<Ktpt>::try_new(vec![Ktpt::default(); 65536]).is_err());
        assert!(checked_section_offset(0x4b).is_err());
        assert_eq!(checked_section_offset(0x4c).unwrap(), 0);
        assert_eq!(checked_section_offset(u64::from(u32::MAX)).unwrap(), u32::MAX - 0x4c);
        assert!(checked_section_offset(u64::from(u32::MAX) + 1).is_err());
    }

    #[test]
    fn maximum_poti_point_count_and_empty_sections_roundtrip() {
        let empty = encode(KmpFile::default());
        assert_eq!(empty.len(), 0x4c + 15 * 8);
        assert_eq!(encode(KmpFile::read(&mut Cursor::new(&empty)).unwrap()), empty);
        let mut kmp = KmpFile::default();
        kmp.poti.entries.push(Poti {
            points: vec![PotiPoint::default(); 65535],
            ..default()
        });
        let bytes = encode(kmp);
        let decoded = KmpFile::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(decoded.poti.section_header.additional_value, 65535);
        assert_eq!(decoded.poti[0].num_points, 65535);
        assert_eq!(encode(decoded), bytes);
    }

    #[test]
    fn binrw_traits_use_validated_canonical_io() {
        let mut output = Cursor::new(Vec::new());
        BinWrite::write(&KmpFile::default(), &mut output).unwrap();
        assert_eq!(u32_at(output.get_ref(), 4) as usize, output.get_ref().len());
        output.set_position(0);
        let kmp: KmpFile = BinRead::read(&mut output).unwrap();
        assert!(kmp.ktpt.is_empty());
    }
}
