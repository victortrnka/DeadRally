//! What a race reads (spec M4 §3): the tracks' archives `TR0.BPA`..`TR9.BPA`, `ENGINE.BPA`
//! and `IBFILES.BPA`, kept whole and decoded when a race starts, and the track decoded from
//! its archive (`loadCircuitInfFile` 0x409BF0, `loadCircuitPalette` 0x402CF0,
//! `loadCircuitImages1` 0x402EE0).

use std::fmt;

use crate::bpa::{Archive, BpaError};
use crate::image::{Image, Palette};
use crate::track::{self, TrackError, TrackInfo};

/// The race's archives.
#[derive(Debug)]
pub struct RaceArchives {
    /// `TR0.BPA` to `TR9.BPA` by number.
    pub tracks: Vec<Archive>,
    pub engine: Archive,
    pub ib_files: Archive,
    /// `MUSICS.BPA`: each track's music and the race's sounds.
    pub musics: Archive,
    /// The cars' handling tables from `dr.exe`.
    pub handling: crate::handling::HandlingTables,
}

#[derive(Debug)]
pub enum RaceError {
    Bpa(BpaError),
    Sound(crate::sound::SoundError),
    Track { name: String, error: TrackError },
}

impl fmt::Display for RaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RaceError::Bpa(error) => write!(f, "{error}"),
            RaceError::Sound(error) => write!(f, "{error}"),
            RaceError::Track { name, error } => write!(f, "{name}: {error}"),
        }
    }
}

impl std::error::Error for RaceError {}

impl From<BpaError> for RaceError {
    fn from(error: BpaError) -> RaceError {
        RaceError::Bpa(error)
    }
}

/// A track: its `-INF.BIN`, its picture `-IMA` with the palette in it, its surface mask
/// `-MAS` (the low nibble a surface's kind), `-LIT.TAB`, the colour each colour turns in the
/// cars' headlights, and its shadows `-SHA`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub info: TrackInfo,
    pub image: Image,
    pub palette: Palette,
    pub mask: Image,
    pub lit: [u8; 256],
    /// What the tires' skid marks and bloody tracks turn the track's colours into
    /// (`-SKI.TAB` at 0x501AA0, `-BLO.TAB` at 0x479D40).
    pub skid: [u8; 256],
    pub blood: [u8; 256],
    pub shadows: Shadows,
    pub scene: Scene,
}

impl Track {
    /// The track turned half round for a reversed circuit (`calculateCircuitReversed`
    /// 0x40A9A0), with its reversed palette `flip` (`-FLIP.PAL`, which `loadCircuitPalette`
    /// 0x402CF0 reads in place of the picture's): the picture and the mask read backwards; the
    /// pedestrians' (16 square, from their corner), the power-ups' spots and the shadows' points
    /// across from where they were; the scene's objects and pictures likewise, their points
    /// turned round their places and the pictures' pixels read backwards.
    #[must_use]
    pub fn reversed(mut self, flip: Palette) -> Track {
        let width = self.info.width as i32;
        let height = self.info.height as i32;
        self.palette = flip;
        self.image.pixels.reverse();
        self.mask.pixels.reverse();
        for spot in &mut self.info.power_ups {
            if spot[0] > 0 {
                spot[0] = width - spot[0] - 1;
            }
            if spot[1] > 0 {
                spot[1] = height - spot[1] - 1;
            }
        }
        for person in &mut self.info.pedestrians {
            if person[0] > 0 {
                person[0] = width - person[0] - 17;
            }
            if person[1] > 0 {
                person[1] = height - person[1] - 17;
            }
        }
        for point in &mut self.shadows.points {
            *point = (width - point.0 - 1, height - point.1 - 1);
        }
        for object in &mut self.scene.objects {
            for point in &mut object.points {
                point[0] = point[0].wrapping_neg();
                point[1] = point[1].wrapping_neg();
            }
            let [least_x, most_x, least_y, most_y] = object.bounds;
            object.bounds = [
                most_x.wrapping_neg(),
                least_x.wrapping_neg(),
                most_y.wrapping_neg(),
                least_y.wrapping_neg(),
            ];
            object.position = (
                width - object.position.0 - 1,
                height - object.position.1 - 1,
            );
        }
        for picture in &mut self.scene.textures {
            let across = (width - 1) * 256;
            let down = (height - 1) * 256;
            picture.position = (
                across - picture.position.0 - picture.width.wrapping_mul(picture.depth),
                down - picture.position.1 - picture.height.wrapping_mul(picture.depth),
            );
        }
        for picture in &self.scene.textures {
            let len = picture.width.wrapping_mul(picture.height);
            let start = usize::try_from(picture.offset).unwrap_or(0);
            let end = start.saturating_add(usize::try_from(len).unwrap_or(0));
            if let Some(pixels) = self.scene.pixels.get_mut(start..end) {
                pixels.reverse();
            }
        }
        self
    }
}

/// A track's shadows (`-SHA.BPK`, read by 0x4034F0): triangles between points of the track
/// that darken what passes under them. The stream holds the counts of points and triangles,
/// the points' x, y and an unused third coordinate, then the triangles' three corners.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shadows {
    pub points: Vec<(i32, i32)>,
    pub triangles: Vec<[usize; 3]>,
}

impl Shadows {
    /// # Errors
    ///
    /// [`TrackError::Mismatch`] when the stream is short or a triangle names a missing point.
    pub fn parse(bytes: &[u8]) -> Result<Shadows, TrackError> {
        let value = |index: usize| {
            bytes
                .get(4 * index..4 * index + 4)
                .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| TrackError::Mismatch("the shadows end early".into()))
        };
        let count = |index: usize| {
            usize::try_from(value(index)?)
                .map_err(|_| TrackError::Mismatch("a negative count of shadows".into()))
        };
        let (points, triangles) = (count(0)?, count(1)?);
        let corner = |index: usize| {
            let point = usize::try_from(value(index)?).ok().filter(|&p| p < points);
            point.ok_or_else(|| TrackError::Mismatch("a shadow names a missing point".into()))
        };
        let corners = 2 + 3 * points;
        Ok(Shadows {
            points: (0..points)
                .map(|i| Ok((value(2 + i)?, value(2 + points + i)?)))
                .collect::<Result<_, TrackError>>()?,
            triangles: (0..triangles)
                .map(|i| {
                    let at = |k: usize| corner(corners + k * triangles + i);
                    Ok([at(0)?, at(1)?, at(2)?])
                })
                .collect::<Result<_, TrackError>>()?,
        })
    }
}

impl Track {
    /// The track `TRn` from its archive.
    ///
    /// # Errors
    ///
    /// [`RaceError`] naming the entry that is missing or does not decode.
    pub fn load(archive: &Archive, number: usize) -> Result<Track, RaceError> {
        let name = |suffix: &str| format!("TR{number}-{suffix}");
        let decode = |suffix: &str, width: u32, height: u32| {
            let entry = name(suffix);
            track::decode_rix3(archive.read(&entry)?, width, height)
                .map_err(|error| RaceError::Track { name: entry, error })
        };
        let info_name = name("INF.BIN");
        let info =
            TrackInfo::parse(archive.read(&info_name)?).map_err(|error| RaceError::Track {
                name: info_name,
                error,
            })?;
        let (image, palette) = decode("IMA.BPK", info.width, info.height)?;
        let (mask, _) = decode("MAS.BPK", info.width, info.height)?;
        // Read into a table of 256 as the original reads it (0x4A9EE0).
        let table = |suffix: &str| -> Result<[u8; 256], RaceError> {
            let mut table = [0; 256];
            let bytes = archive.read(&name(suffix))?;
            let len = bytes.len().min(256);
            table[..len].copy_from_slice(&bytes[..len]);
            Ok(table)
        };
        let lit = table("LIT.TAB")?;
        let skid = table("SKI.TAB")?;
        let blood = table("BLO.TAB")?;
        let scene_name = name("SCE.BPK");
        // The first two tracks have room for more texture pixels (the jump table at 0x4032BC).
        let pixels = if matches!(number, 1 | 2) {
            0x5F370
        } else {
            0x493E0
        };
        let scene = crate::bpk::decode(archive.read(&scene_name)?)
            .map_err(TrackError::Bpk)
            .and_then(|bytes| Scene::parse(&bytes, pixels))
            .map_err(|error| RaceError::Track {
                name: scene_name,
                error,
            })?;
        let shadows_name = name("SHA.BPK");
        let shadows = crate::bpk::decode(archive.read(&shadows_name)?)
            .map_err(TrackError::Bpk)
            .and_then(|bytes| Shadows::parse(&bytes))
            .map_err(|error| RaceError::Track {
                name: shadows_name,
                error,
            })?;
        Ok(Track {
            info,
            image,
            palette,
            mask,
            lit,
            skid,
            blood,
            shadows,
            scene,
        })
    }
}

/// A track's 3D scene (`-SCE.BPK`, read by `parseCircuitSceFile` 0x403190): the buildings
/// and walls drawn over the track in perspective, and the pictures stuck on them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    pub textures: Vec<SceneTexture>,
    /// Every texture's pixels, each at its `offset`.
    pub pixels: Vec<u8>,
}

/// An object of the scene, 3152 bytes in the file: its points relative to `position` (x and
/// y in 256ths of a pixel, z its depth, 256 on the track), its triangles, and the box its
/// points fill (`bounds`: least x, most x, least y, most y, in 256ths).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneObject {
    pub points: Vec<[i32; 3]>,
    pub triangles: Vec<SceneTriangle>,
    pub bounds: [i32; 4],
    pub position: (i32, i32),
}

/// A triangle of an object: its corners and its colour, 0x80 to 0x8A for the shaded kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneTriangle {
    pub corners: [usize; 3],
    pub colour: i32,
}

/// A picture on the scene (44 bytes in the file): its size, where its pixels start, where it
/// is on the track (in 256ths) and its depth, and the object and triangle it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneTexture {
    pub width: i32,
    pub height: i32,
    pub offset: i32,
    pub position: (i32, i32),
    pub depth: i32,
    pub object: i32,
    pub triangle: i32,
}

/// An object's size in the file, and the most points and triangles it has room for.
const OBJECT_BYTES: usize = 0xC50;
const TEXTURE_BYTES: usize = 0x2C;
pub const MAX_POINTS: usize = 75;
pub const MAX_TRIANGLES: usize = 100;

impl Scene {
    /// The scene from its decoded stream: a byte counting the objects, the objects, a byte
    /// counting the textures, the textures, then `pixels` bytes of their pixels (300000, or
    /// 390000 on the first two tracks).
    ///
    /// # Errors
    ///
    /// [`TrackError::Mismatch`] when the stream is short or an object has more points or
    /// triangles than the original has room for, or a triangle names a missing point.
    pub fn parse(bytes: &[u8], pixels: usize) -> Result<Scene, TrackError> {
        let short = || TrackError::Mismatch("the scene ends early".into());
        let objects = usize::from(*bytes.first().ok_or_else(short)?);
        let textures_at = 1 + objects * OBJECT_BYTES;
        let textures = usize::from(*bytes.get(textures_at).ok_or_else(short)?);
        let int = |at: usize| {
            bytes
                .get(at..at + 4)
                .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(short)
        };
        let count = |at: usize, most: usize| {
            usize::try_from(int(at)?)
                .ok()
                .filter(|&n| n <= most)
                .ok_or_else(|| TrackError::Mismatch("a scene object is too big".into()))
        };
        let objects = (0..objects)
            .map(|index| {
                let base = 1 + index * OBJECT_BYTES;
                let field = |offset: usize, i: usize| int(base + offset + 4 * i);
                let points = count(base, MAX_POINTS)?;
                let triangles = count(base + 4, MAX_TRIANGLES)?;
                let corner = |offset: usize, i: usize| {
                    usize::try_from(field(offset, i)?)
                        .ok()
                        .filter(|&p| p < points)
                        .ok_or_else(|| {
                            TrackError::Mismatch("a scene triangle names a missing point".into())
                        })
                };
                Ok(SceneObject {
                    points: (0..points)
                        .map(|i| Ok([field(0x8, i)?, field(0x134, i)?, field(0x260, i)?]))
                        .collect::<Result<_, TrackError>>()?,
                    triangles: (0..triangles)
                        .map(|i| {
                            Ok(SceneTriangle {
                                corners: [corner(0x39C, i)?, corner(0x52C, i)?, corner(0x6BC, i)?],
                                colour: field(0x84C, i)?,
                            })
                        })
                        .collect::<Result<_, TrackError>>()?,
                    bounds: [
                        field(0x38C, 0)?,
                        field(0x38C, 1)?,
                        field(0x38C, 2)?,
                        field(0x38C, 3)?,
                    ],
                    position: (field(0x9DC, 0)?, field(0x9E0, 0)?),
                })
            })
            .collect::<Result<_, TrackError>>()?;
        let textures = (0..textures)
            .map(|index| {
                let field = |i: usize| int(textures_at + 1 + index * TEXTURE_BYTES + 4 * i);
                Ok(SceneTexture {
                    width: field(0)?,
                    height: field(1)?,
                    offset: field(2)?,
                    position: (field(3)?, field(4)?),
                    depth: field(5)?,
                    object: field(9)?,
                    triangle: field(10)?,
                })
            })
            .collect::<Result<Vec<_>, TrackError>>()?;
        let start = textures_at + 1 + textures.len() * TEXTURE_BYTES;
        let mut pixels_bytes = bytes.get(start..).unwrap_or(&[]).to_vec();
        pixels_bytes.resize(pixels, 0);
        Ok(Scene {
            objects,
            textures,
            pixels: pixels_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// The shadows' points come as all their x, then all their y (a third coordinate after is
    /// skipped), and the triangles as all their first corners, then second, then third: read
    /// in another order the shadows would fall across the wrong parts of the track.
    #[test]
    fn shadows_read_their_points_and_corners_column_by_column() {
        let bytes = stream(&[
            3, 2, 10, 20, 30, 11, 21, 31, 0, 0, 0, 0, 1, 2, 0, 1, 2, 2, 0,
        ]);
        let shadows = Shadows::parse(&bytes).unwrap();
        assert_eq!(shadows.points, [(10, 11), (20, 21), (30, 31)]);
        assert_eq!(shadows.triangles, [[0, 2, 1], [1, 0, 2]]);
    }

    /// A reversed circuit's track is the same track turned half round: what stood at (x, y)
    /// stands at (width - 1 - x, height - 1 - y), pedestrians (16 square, placed by their
    /// corner) at 17 less, spots left empty (0) stay empty, the scene's points turn round their
    /// objects' places, and the pictures' pixels read backwards.
    #[test]
    fn a_reversed_track_is_turned_half_round() {
        let mut info = TrackInfo {
            width: 100,
            height: 50,
            zones: 0,
            starts: [[0; 3]; 4],
            power_ups: [[0; 2]; 16],
            pedestrians: [[0; 4]; 20],
        };
        info.power_ups[0] = [10, 20];
        info.pedestrians[0] = [10, 20, 1, 2];
        let picture = |pixels: Vec<u8>| Image {
            width: 3,
            height: 1,
            pixels,
        };
        let track = Track {
            info,
            image: Image {
                width: 100,
                height: 50,
                pixels: (0..5000).map(|i| (i % 251) as u8).collect(),
            },
            palette: Palette::BLACK,
            mask: picture(vec![1, 2, 3]),
            lit: [0; 256],
            skid: [0; 256],
            blood: [0; 256],
            shadows: Shadows {
                points: vec![(0, 0)],
                triangles: vec![],
            },
            scene: Scene {
                objects: vec![SceneObject {
                    points: vec![[256, -512, 300]],
                    triangles: vec![],
                    bounds: [-512, 256, -768, 0],
                    position: (30, 40),
                }],
                textures: vec![SceneTexture {
                    width: 2,
                    height: 1,
                    offset: 1,
                    position: (2560, 5120),
                    depth: 256,
                    object: 0,
                    triangle: 0,
                }],
                pixels: vec![9, 1, 2, 9],
            },
        };
        let first = track.image.pixels[0];
        let mut flip = Palette::BLACK;
        flip.0[1] = [1, 2, 3];
        let turned = track.reversed(flip.clone());
        assert_eq!(turned.palette, flip);
        assert_eq!(*turned.image.pixels.last().unwrap(), first);
        assert_eq!(turned.mask.pixels, [3, 2, 1]);
        assert_eq!(turned.info.power_ups[0], [89, 29]);
        assert_eq!(turned.info.power_ups[1], [0, 0]);
        assert_eq!(&turned.info.pedestrians[0][..2], &[73, 13]);
        assert_eq!(turned.shadows.points, [(99, 49)]);
        let object = &turned.scene.objects[0];
        assert_eq!(object.points, [[-256, 512, 300]]);
        assert_eq!(object.bounds, [-256, 512, 0, 768]);
        assert_eq!(object.position, (69, 9));
        assert_eq!(
            turned.scene.textures[0].position,
            (99 * 256 - 2560 - 512, 49 * 256 - 5120 - 256)
        );
        assert_eq!(turned.scene.pixels, [9, 2, 1, 9]);
    }

    #[test]
    fn a_shadow_naming_a_missing_point_is_refused() {
        let bytes = stream(&[1, 1, 10, 11, 0, 0, 0, 1]);
        assert!(Shadows::parse(&bytes).is_err());
    }
}
