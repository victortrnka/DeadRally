//! The decoded data the startup sequence and the main menu need (spec M1a §4.2, M1b §4.1,
//! M2a §4.1).

use std::fmt;
use std::path::PathBuf;

use crate::bmp::{self, BmpError};
use crate::bpa::{Archive, BpaError};
use crate::catalog::{self, CatalogError};
use crate::dr_cfg::DrCfg;
use crate::exe::{Exe, ExeError};
use crate::haf::{Animation, HafError};
use crate::handling::HandlingTables;
use crate::image::{Image, Palette, PaletteError};
use crate::machine::MachineError;
use crate::s3m::Module;
use crate::sound::{self, SoundError};
use crate::text::{TextError, Texts};
use crate::validate::Validation;
use crate::xm::Bank;

/// An image with the palette it is shown with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub image: Image,
    pub palette: Palette,
}

/// Everything the startup sequence shows.
#[derive(Debug)]
pub struct Assets {
    /// `SANIM.haf`.
    pub intro: Animation,
    /// `FRAMES.BPK`: the intro's 320x200 letterbox; the game uses palette entries 0..=15.
    pub letterbox: Picture,
    /// `APOGEE.BPK` with `APOGEE.PAL`.
    pub apogee: Picture,
    /// `rmd.bmp`.
    pub remedy: Picture,
    /// `STARTSCR.BPK` with `STARTSCR.PAL`.
    pub title: Picture,
    /// `TR0-MUS.CMF`: the music under the intro.
    pub intro_music: Module,
    /// `SANIM-E.CMF`: the intro's effects, numbered as in `SANIM.haf`'s table.
    pub intro_effects: Bank,
    /// `MEN-MUS.CMF`: the music that starts when the intro ends and goes on into the menus.
    pub menu_music: Module,
    /// `ENDANI0.HAF` with `ENDANI0E.CMF`: the Adversary's animation when a player first leads
    /// (spec M6); `ENDANI.HAF` with `ENDANI-E.CMF`: the end. Both play under `TR0-MUS.CMF`.
    pub adversary_animation: Animation,
    pub adversary_effects: Bank,
    pub end_animation: Animation,
    pub end_effects: Bank,
    pub menu: MenuAssets,
    pub race: crate::race::RaceArchives,
}

/// What the main menu draws and plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAssets {
    /// `MENUBG5.BPK`, 640x480.
    pub background: Image,
    /// `CHATLIN1.BPK`, 640x10: the bottom panel's frame lines.
    pub panel_line: Image,
    /// `CORN3A.BPK` and `CORN3B.BPK`: popup corners (top left, top right, bottom left, bottom
    /// right) of a focused and of an unfocused popup.
    pub corners_focused: Vec<Image>,
    pub corners_unfocused: Vec<Image>,
    /// `CURSOR.BPK`: 50 frames, 20x20.
    pub cursor: Vec<Image>,
    /// `F-BIG3A`, `-B`, `-D` (32x32) and `F-SMA3A`, `-B`, `-C` (16x16): 96 glyphs each.
    pub big_a: Vec<Image>,
    pub big_b: Vec<Image>,
    pub big_d: Vec<Image>,
    pub small_a: Vec<Image>,
    pub small_b: Vec<Image>,
    pub small_c: Vec<Image>,
    /// `MENU.PAL`.
    pub palette: Palette,
    /// `COPPER.PAL`: one colour per player colour, whose ramps the menu palette gets.
    pub copper: Palette,
    /// `CARCOL.PAL`: the other drivers' car colours in a race.
    pub car_colours: Palette,
    /// `BGCOP.PAL`: 512 colours for the background copper rows.
    pub background_copper: Vec<[u8; 3]>,
    /// `CREDIT1.BPK` and `CREDIT2.BPK` with their palettes.
    pub credits: Vec<Picture>,
    /// `end.bmp`: the screen the game ends on.
    pub end: Picture,
    /// `MEN-SAM.CMF`: the menus' effects.
    pub effects: Bank,
    /// The strings and font metrics in `dr.exe`.
    pub texts: Texts,
    /// `SLIDMUS2.BPK` and `VOLCUR2.BPK`: the volume popups' slider and its knob.
    pub slider: Image,
    pub knob: Image,
    /// The `dr.cfg` the original writes when it has none, from `dr.exe`'s `defaultConfig`.
    pub default_config: DrCfg,
    /// The Hall of Fame (spec M2c §3): `F-MED1A` (62 glyphs), `FAMETXT`, `RECOTXT`, `RECOBAR`,
    /// the circuits' snapshots (`TRSNAP2M`), the arrows (`TRARR1`), the wipe's masks
    /// (`15X150`) and the border's corners (`CHOO2`).
    pub medium: Vec<Image>,
    pub fame_title: Image,
    pub records_title: Image,
    pub records_bar: Image,
    pub snapshots: Vec<Image>,
    /// DeadRally's snapshot of the Arena, which `TRSNAP2M` has none of ([`arena_snapshot`]).
    pub arena_snapshot: Image,
    /// DeadRally's font for titles as the records' "TRACK RECORDS" is lettered, every
    /// printable character ([`title_font`]): the glyphs and the pen's advance after each.
    pub title_glyphs: Vec<Image>,
    pub title_advances: Vec<u8>,
    pub arrows: Vec<Image>,
    pub wipe: Vec<Image>,
    pub border_corners: Vec<Image>,
    /// The driver's licence (spec M3a §3): `LICENCE3`, the face's frame `FACESEL1`, the faces
    /// `FACE01`–`FACE20`, the face arrows `FACEARR1` (up, down, up lit, down lit), the car's box
    /// `CARBAS2`, the car names `CARNAME`, the cars turning (64 frames each, car 0's
    /// `KUPLA` on the licence), the colour slider `SLIDCOP2` and its knob `SLIDCUR3`, and the
    /// price digits `F-SMA3F` ("$", then 0 to 9).
    pub licence: Image,
    pub face_frame: Image,
    pub faces: Vec<Image>,
    pub face_arrows: Vec<Image>,
    pub car_box: Image,
    pub car_names: Vec<Image>,
    pub car_turning: Vec<Vec<Image>>,
    pub colour_slider: Image,
    pub colour_knob: Image,
    pub price_digits: Vec<Image>,
    /// The race's results' pictures.
    pub results: ResultsPictures,
    /// The Adversary's screen (spec M6): its title `BADTIT`, the Adversary `BADSNAP` and
    /// the box saying Escape backs out `ESCBOX`.
    pub adversary_title: Image,
    pub adversary: Image,
    pub escape_box: Image,
    /// The sign-up (spec M3a §3): its title `ENTERTX2`, the side panel
    /// `STATBAS7` with the cars `SCENECAR` and the upgrade lamps `STATPOP4`, and the line under
    /// the player's entry `SIGNLINE`.
    pub sign_up_title: Image,
    pub side_panel: Image,
    pub side_cars: Vec<Image>,
    pub upgrade_lamps: Vec<Image>,
    pub sign_line: Image,
    /// The shop (spec M3b §3): its title `SHOPTXT1`, the car box's arrows `ARROWS1D` (left,
    /// right, left lit, right lit), the five items' boxes `BASES4`, the engines `ENGI1`–`4`,
    /// tires `TIRE1`–`4` and armours `ARMOR1`–`4` by level, the repair `REPAANI` and the
    /// continue `CONTANI` turning, and the maxed-out pictures `MAXI1F`.
    pub shop_title: Image,
    pub car_arrows: Vec<Image>,
    pub item_boxes: Vec<Image>,
    pub engines: Vec<Vec<Image>>,
    pub tires: Vec<Vec<Image>>,
    pub armours: Vec<Vec<Image>>,
    pub repair: Vec<Image>,
    pub continue_flag: Vec<Image>,
    pub maxed: Vec<Image>,
    /// The Underground Market (spec M3c §3): its title `BLACKTX1`, the loan shark `DEALER2B`,
    /// the weapons `MARKET1E` (mines, spikes, rocket fuel, sabotage; locked; out of stock),
    /// and `market_prices[car]`, the four weapons' prices with that car, as
    /// `setUndergroundMarketPrices` (0x421FB0) sets them.
    pub market_title: Image,
    pub loan_shark: Image,
    pub weapons: Vec<Image>,
    pub market_prices: Vec<[i32; 4]>,
    /// The offers after a sign-up (0x431B30): the drug dealer `DRUGDEAL`, the hitman
    /// `EVENT_2`.
    pub drug_dealer: Image,
    pub hitman: Image,
    /// The shop's popups after a race (spec M5): the sponsor `SPONS1B` and the reaper
    /// `REAPER_X`.
    pub sponsor: Image,
    pub reaper: Image,
    /// The race's preview (spec M4 §3): the banner `PREP4`, the grid's frame `PREPW1` and the
    /// circuits' pictures `TSHAPE01`..`TSHAPE19`, by circuit (the last the Adversary's).
    pub preview_banner: Image,
    pub preview_grid: Image,
    pub track_shapes: Vec<Image>,
    /// The Adversary's face in the Arena's preview (`FACEXX`, loaded at 0x41962D).
    pub adversary_face: Image,
}

#[derive(Debug)]
pub enum AssetError {
    Archive(BpaError),
    Image {
        name: &'static str,
        error: CatalogError,
    },
    Palette {
        name: &'static str,
        error: PaletteError,
    },
    Bmp {
        path: PathBuf,
        error: BmpError,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Animation(HafError),
    Sound(SoundError),
    Exe(ExeError),
    Machine(MachineError),
    Text(TextError),
    /// The Arena's track, of which DeadRally makes its snapshot.
    Track(crate::race::RaceError),
    Size {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Archive(error) => write!(f, "{error}"),
            AssetError::Image { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Palette { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Bmp { path, error } => write!(f, "{}: {error}", path.display()),
            AssetError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            AssetError::Animation(error) => write!(f, "{error}"),
            AssetError::Sound(error) => write!(f, "{error}"),
            AssetError::Exe(error) => write!(f, "dr.exe: {error}"),
            AssetError::Machine(error) => write!(f, "{error}"),
            AssetError::Text(error) => write!(f, "{error}"),
            AssetError::Track(error) => write!(f, "{error}"),
            AssetError::Size {
                name,
                expected,
                actual,
            } => write!(f, "MENU.BPA/{name}: {actual} bytes, not {expected}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<BpaError> for AssetError {
    fn from(error: BpaError) -> AssetError {
        AssetError::Archive(error)
    }
}

impl Assets {
    /// Loads the startup sequence's data from a validated data directory.
    ///
    /// # Errors
    ///
    /// [`AssetError`] naming the file and entry that failed.
    pub fn load(validation: &Validation) -> Result<Assets, AssetError> {
        let path = |name: &str| -> PathBuf {
            validation
                .files
                .iter()
                .find(|file| file.name == name)
                .map(|file| file.path.clone())
                .unwrap_or_else(|| panic!("{name} is a required file, so validation found it"))
        };
        let menu = Archive::open(&path("MENU.BPA"))?;
        let musics = Archive::open(&path(sound::ARCHIVE))?;
        let exe_path = path("DR.EXE");
        let exe_bytes = std::fs::read(&exe_path).map_err(|source| AssetError::Read {
            path: exe_path,
            source,
        })?;
        let exe = Exe::parse(exe_bytes).map_err(AssetError::Exe)?;
        let remedy_path = path("RMD.BMP");
        let remedy_bytes = std::fs::read(&remedy_path).map_err(|source| AssetError::Read {
            path: remedy_path.clone(),
            source,
        })?;
        let (image, palette) = bmp::decode(&remedy_bytes).map_err(|error| AssetError::Bmp {
            path: remedy_path,
            error,
        })?;
        let tracks: Vec<Archive> = (0..10)
            .map(|n| Archive::open(&path(&format!("TR{n}.BPA"))))
            .collect::<Result<_, _>>()?;
        let mut menu_assets = menu_assets(&menu, &musics, &path("END.BMP"), &exe)?;
        let arena = crate::race::Track::load(&tracks[0], 0).map_err(AssetError::Track)?;
        (menu_assets.title_glyphs, menu_assets.title_advances) = title_font(
            &menu_assets.records_title,
            &menu_assets.background,
            &menu_assets.palette,
        );
        menu_assets.arena_snapshot = arena_snapshot(
            &menu_assets.snapshots,
            &menu_assets.palette,
            (&arena.image, &arena.palette),
        );
        Ok(Assets {
            intro: Animation::open(&path("SANIM.HAF")).map_err(AssetError::Animation)?,
            letterbox: letterbox(&menu)?,
            apogee: picture(&menu, "APOGEE.BPK", "APOGEE.PAL")?,
            remedy: Picture { image, palette },
            title: picture(&menu, "STARTSCR.BPK", "STARTSCR.PAL")?,
            intro_music: sound::load_music(&musics, "TR0-MUS.CMF").map_err(AssetError::Sound)?,
            intro_effects: sound::load_effects(&musics, "SANIM-E.CMF")
                .map_err(AssetError::Sound)?,
            menu_music: sound::load_music(&musics, "MEN-MUS.CMF").map_err(AssetError::Sound)?,
            adversary_animation: Animation::open(&path("ENDANI0.HAF"))
                .map_err(AssetError::Animation)?,
            adversary_effects: sound::load_effects(&musics, "ENDANI0E.CMF")
                .map_err(AssetError::Sound)?,
            end_animation: Animation::open(&path("ENDANI.HAF")).map_err(AssetError::Animation)?,
            end_effects: sound::load_effects(&musics, "ENDANI-E.CMF").map_err(AssetError::Sound)?,
            menu: menu_assets,
            race: crate::race::RaceArchives {
                tracks,
                engine: Archive::open(&path("ENGINE.BPA"))?,
                ib_files: Archive::open(&path("IBFILES.BPA"))?,
                musics,
                handling: HandlingTables::read(&exe).map_err(AssetError::Machine)?,
            },
        })
    }
}

/// The race's results (spec M5): the ranking `RANK1C`, the results' panel `RESUPOK1`, the
/// place's box `PLACING`, the races' titles `RACE-EAS`, `-MED`, `-HAR` and points `PTS-EASY`,
/// `-MED`, `-HARD`, the standings' row pieces for the player `P-RANK`, `P-NAME`, `P-PIST` and
/// the others `RANKINF1`–`3`, and the cars `CARRES` (a picture for each car in each place).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultsPictures {
    pub ranking: Image,
    pub panel: Image,
    pub placing: Image,
    pub races: Vec<Image>,
    pub points: Vec<Image>,
    pub player_row: Vec<Image>,
    pub other_row: Vec<Image>,
    pub cars: Vec<Image>,
}

/// All frames of a catalogued `MENU.BPA` image.
fn frames(menu: &Archive, name: &'static str) -> Result<Vec<Image>, AssetError> {
    let entry = catalog::find("MENU.BPA", name).expect("menu images are catalogued");
    entry
        .decode(menu.read(name)?)
        .map_err(|error| AssetError::Image { name, error })
}

fn palette(menu: &Archive, name: &'static str) -> Result<Palette, AssetError> {
    Palette::from_bytes(menu.read(name)?).map_err(|error| AssetError::Palette { name, error })
}

fn bmp_picture(path: &std::path::Path) -> Result<Picture, AssetError> {
    let bytes = std::fs::read(path).map_err(|source| AssetError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let (image, palette) = bmp::decode(&bytes).map_err(|error| AssetError::Bmp {
        path: path.to_path_buf(),
        error,
    })?;
    Ok(Picture { image, palette })
}

/// A picture the menu copies over its whole screen, which must be 640x480.
fn full_screen(picture: Picture, path: &std::path::Path) -> Result<Picture, AssetError> {
    let (width, height) = (picture.image.width, picture.image.height);
    if (width, height) == (640, 480) {
        Ok(picture)
    } else {
        Err(AssetError::Bmp {
            path: path.to_path_buf(),
            error: BmpError::Unsupported(format!("{width}x{height}, not 640x480")),
        })
    }
}

/// The Underground Market's prices for each car: `setUndergroundMarketPrices` run with the
/// player (driver 19) in that car, its four prices read where it leaves them.
fn market_prices(exe: &Exe) -> Result<Vec<[i32; 4]>, crate::machine::MachineError> {
    const FUNCTION: (u32, u32) = (0x42_1FB0, 0x42_20C8);
    const DRIVER_ID: u32 = 0x46_3CE8;
    const PLAYER_CAR: u32 = 0x46_085C + 19 * 0x6C;
    const PRICES: u32 = 0x46_2D40;
    (0..6)
        .map(|car| {
            let mut machine = crate::machine::Machine::new(exe);
            machine.poke(DRIVER_ID, 19);
            machine.poke(PLAYER_CAR, car);
            machine.run(FUNCTION.0, FUNCTION.1)?;
            let bytes = machine.bytes(PRICES, 16)?;
            Ok(std::array::from_fn(|w| {
                i32::from_le_bytes([
                    bytes[4 * w],
                    bytes[4 * w + 1],
                    bytes[4 * w + 2],
                    bytes[4 * w + 3],
                ])
            }))
        })
        .collect()
}

/// The circuits' pictures in the race's preview, circuit 0 to 17 and the Adversary's
/// (`previewRaceScreen` 0x4321B0, the names at 0x456798).
const TRACK_SHAPES: [&str; 19] = [
    "TSHAPE01.BPK",
    "TSHAPE02.BPK",
    "TSHAPE03.BPK",
    "TSHAPE04.BPK",
    "TSHAPE05.BPK",
    "TSHAPE06.BPK",
    "TSHAPE07.BPK",
    "TSHAPE08.BPK",
    "TSHAPE09.BPK",
    "TSHAPE10.BPK",
    "TSHAPE11.BPK",
    "TSHAPE12.BPK",
    "TSHAPE13.BPK",
    "TSHAPE14.BPK",
    "TSHAPE15.BPK",
    "TSHAPE16.BPK",
    "TSHAPE17.BPK",
    "TSHAPE18.BPK",
    "TSHAPE19.BPK",
];

/// The cars turning in the menus, car 0 to 5 (dRally `___24548h.c`).
const CAR_TURNING: [&str; 6] = [
    "KUPLA.BPK",
    "PICKUP.BPK",
    "SEDAN.BPK",
    "CAMARO.BPK",
    "PORSCHE.BPK",
    "LOTUS.BPK",
];

/// The twenty drivers' faces, by face.
const FACES: [&str; 20] = [
    "FACE01.BPK",
    "FACE02.BPK",
    "FACE03.BPK",
    "FACE04.BPK",
    "FACE05.BPK",
    "FACE06.BPK",
    "FACE07.BPK",
    "FACE08.BPK",
    "FACE09.BPK",
    "FACE10.BPK",
    "FACE11.BPK",
    "FACE12.BPK",
    "FACE13.BPK",
    "FACE14.BPK",
    "FACE15.BPK",
    "FACE16.BPK",
    "FACE17.BPK",
    "FACE18.BPK",
    "FACE19.BPK",
    "FACE20.BPK",
];

fn menu_assets(
    menu: &Archive,
    musics: &Archive,
    end: &std::path::Path,
    exe: &Exe,
) -> Result<MenuAssets, AssetError> {
    const BGCOP: &str = "BGCOP.PAL";
    let background_copper = menu.read(BGCOP)?;
    // 512 colours of 6-bit components.
    if background_copper.len() != 3 * 512 {
        return Err(AssetError::Size {
            name: BGCOP,
            expected: 3 * 512,
            actual: background_copper.len(),
        });
    }
    if let Some((index, &value)) = background_copper.iter().enumerate().find(|(_, c)| **c > 63) {
        return Err(AssetError::Palette {
            name: BGCOP,
            error: PaletteError::NotSixBit { index, value },
        });
    }
    Ok(MenuAssets {
        background: frames(menu, "MENUBG5.BPK")?.remove(0),
        panel_line: frames(menu, "CHATLIN1.BPK")?.remove(0),
        corners_focused: frames(menu, "CORN3A.BPK")?,
        corners_unfocused: frames(menu, "CORN3B.BPK")?,
        cursor: frames(menu, "CURSOR.BPK")?,
        big_a: frames(menu, "F-BIG3A.BPK")?,
        big_b: frames(menu, "F-BIG3B.BPK")?,
        big_d: frames(menu, "F-BIG3D.BPK")?,
        small_a: frames(menu, "F-SMA3A.BPK")?,
        small_b: frames(menu, "F-SMA3B.BPK")?,
        small_c: frames(menu, "F-SMA3C.BPK")?,
        palette: palette(menu, "MENU.PAL")?,
        copper: palette(menu, "COPPER.PAL")?,
        car_colours: palette(menu, "CARCOL.PAL")?,
        background_copper: background_copper.as_chunks::<3>().0.to_vec(),
        credits: vec![
            picture(menu, "CREDIT1.BPK", "CREDIT1.PAL")?,
            picture(menu, "CREDIT2.BPK", "CREDIT2.PAL")?,
        ],
        end: full_screen(bmp_picture(end)?, end)?,
        effects: sound::load_effects(musics, "MEN-SAM.CMF").map_err(AssetError::Sound)?,
        texts: Texts::read(exe).map_err(AssetError::Text)?,
        slider: frames(menu, "SLIDMUS2.BPK")?.remove(0),
        knob: frames(menu, "VOLCUR2.BPK")?.remove(0),
        default_config: DrCfg::defaults(exe).map_err(AssetError::Machine)?,
        medium: frames(menu, "F-MED1A.BPK")?,
        fame_title: frames(menu, "FAMETXT.BPK")?.remove(0),
        records_title: frames(menu, "RECOTXT.BPK")?.remove(0),
        records_bar: frames(menu, "RECOBAR.BPK")?.remove(0),
        snapshots: frames(menu, "TRSNAP2M.BPK")?,
        arena_snapshot: Image::new(0, 0, Vec::new()),
        title_glyphs: Vec::new(),
        title_advances: Vec::new(),
        arrows: frames(menu, "TRARR1.BPK")?,
        wipe: frames(menu, "15X150.BPK")?,
        border_corners: frames(menu, "CHOO2.BPK")?,
        licence: frames(menu, "LICENCE3.BPK")?.remove(0),
        adversary_title: frames(menu, "BADTIT.BPK")?.remove(0),
        adversary: frames(menu, "BADSNAP.BPK")?.remove(0),
        escape_box: frames(menu, "ESCBOX.BPK")?.remove(0),
        results: ResultsPictures {
            ranking: frames(menu, "RANK1C.BPK")?.remove(0),
            panel: frames(menu, "RESUPOK1.BPK")?.remove(0),
            placing: frames(menu, "PLACING.BPK")?.remove(0),
            races: ["RACE-EAS.BPK", "RACE-MED.BPK", "RACE-HAR.BPK"]
                .into_iter()
                .map(|name| frames(menu, name).map(|mut f| f.remove(0)))
                .collect::<Result<_, _>>()?,
            points: ["PTS-EASY.BPK", "PTS-MED.BPK", "PTS-HARD.BPK"]
                .into_iter()
                .map(|name| frames(menu, name).map(|mut f| f.remove(0)))
                .collect::<Result<_, _>>()?,
            player_row: ["P-RANK.BPK", "P-NAME.BPK", "P-PIST.BPK"]
                .into_iter()
                .map(|name| frames(menu, name).map(|mut f| f.remove(0)))
                .collect::<Result<_, _>>()?,
            other_row: ["RANKINF1.BPK", "RANKINF2.BPK", "RANKINF3.BPK"]
                .into_iter()
                .map(|name| frames(menu, name).map(|mut f| f.remove(0)))
                .collect::<Result<_, _>>()?,
            cars: frames(menu, "CARRES.BPK")?,
        },
        face_frame: frames(menu, "FACESEL1.BPK")?.remove(0),
        faces: FACES
            .iter()
            .map(|&name| Ok(frames(menu, name)?.remove(0)))
            .collect::<Result<_, AssetError>>()?,
        face_arrows: frames(menu, "FACEARR1.BPK")?,
        car_box: frames(menu, "CARBAS2.BPK")?.remove(0),
        car_names: frames(menu, "CARNAME.BPK")?,
        car_turning: CAR_TURNING
            .iter()
            .map(|&name| frames(menu, name))
            .collect::<Result<_, _>>()?,
        colour_slider: frames(menu, "SLIDCOP2.BPK")?.remove(0),
        colour_knob: frames(menu, "SLIDCUR3.BPK")?.remove(0),
        price_digits: frames(menu, "F-SMA3F.BPK")?,
        sign_up_title: frames(menu, "ENTERTX2.BPK")?.remove(0),
        side_panel: frames(menu, "STATBAS7.BPK")?.remove(0),
        side_cars: frames(menu, "SCENECAR.BPK")?,
        upgrade_lamps: frames(menu, "STATPOP4.BPK")?,
        sign_line: frames(menu, "SIGNLINE.BPK")?.remove(0),
        shop_title: frames(menu, "SHOPTXT1.BPK")?.remove(0),
        car_arrows: frames(menu, "ARROWS1D.BPK")?,
        item_boxes: frames(menu, "BASES4.BPK")?,
        engines: ["ENGI1.BPK", "ENGI2.BPK", "ENGI3.BPK", "ENGI4.BPK"]
            .iter()
            .map(|&name| frames(menu, name))
            .collect::<Result<_, _>>()?,
        tires: ["TIRE1.BPK", "TIRE2.BPK", "TIRE3.BPK", "TIRE4.BPK"]
            .iter()
            .map(|&name| frames(menu, name))
            .collect::<Result<_, _>>()?,
        armours: ["ARMOR1.BPK", "ARMOR2.BPK", "ARMOR3.BPK", "ARMOR4.BPK"]
            .iter()
            .map(|&name| frames(menu, name))
            .collect::<Result<_, _>>()?,
        repair: frames(menu, "REPAANI.BPK")?,
        continue_flag: frames(menu, "CONTANI.BPK")?,
        maxed: frames(menu, "MAXI1F.BPK")?,
        market_title: frames(menu, "BLACKTX1.BPK")?.remove(0),
        loan_shark: frames(menu, "DEALER2B.BPK")?.remove(0),
        weapons: frames(menu, "MARKET1E.BPK")?,
        market_prices: market_prices(exe).map_err(AssetError::Machine)?,
        drug_dealer: frames(menu, "DRUGDEAL.BPK")?.remove(0),
        hitman: frames(menu, "EVENT_2.BPK")?.remove(0),
        sponsor: frames(menu, "SPONS1B.BPK")?.remove(0),
        reaper: frames(menu, "REAPER_X.BPK")?.remove(0),
        preview_banner: frames(menu, "PREP4.BPK")?.remove(0),
        preview_grid: frames(menu, "PREPW1.BPK")?.remove(0),
        track_shapes: TRACK_SHAPES
            .iter()
            .map(|name| Ok(frames(menu, name)?.remove(0)))
            .collect::<Result<_, AssetError>>()?,
        adversary_face: frames(menu, "FACEXX.BPK")?.remove(0),
    })
}

fn picture(
    menu: &Archive,
    image: &'static str,
    palette: &'static str,
) -> Result<Picture, AssetError> {
    let entry = catalog::find("MENU.BPA", image).expect("these images are catalogued");
    let mut frames = entry
        .decode(menu.read(image)?)
        .map_err(|error| AssetError::Image { name: image, error })?;
    let palette =
        Palette::from_bytes(menu.read(palette)?).map_err(|error| AssetError::Palette {
            name: palette,
            error,
        })?;
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

fn letterbox(menu: &Archive) -> Result<Picture, AssetError> {
    const NAME: &str = "FRAMES.BPK";
    let entry = catalog::find("MENU.BPA", NAME).expect("FRAMES.BPK is catalogued");
    let bytes = menu.read(NAME)?;
    let image_error = |error| AssetError::Image { name: NAME, error };
    let mut frames = entry.decode(bytes).map_err(image_error)?;
    let palette = entry
        .embedded_palette(bytes)
        .map_err(image_error)?
        .expect("FRAMES.BPK embeds its palette");
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

/// The snapshots' parts: the eighteen circuits' frames (two more for the sign-up's buttons
/// follow), their name's strip (lines 0 to 13) whose first eight columns hold one turn of its
/// pattern and no letters, and their picture (columns 3 to 124, lines 18 to 94).
const CIRCUIT_SNAPSHOTS: usize = 18;
const NAME_STRIP: (usize, usize) = (0, 14);
const STRIP_TILE: (usize, usize) = (3, 8);
const PICTURE: (usize, usize, usize, usize) = (3, 18, 122, 77);
/// The part of the Arena's track (`TR0-IMA`, 1408 x 512) its picture shows: a race's view,
/// 320 x 202, round the start line, made smaller as the circuits' pictures are.
const ARENA_VIEW: (usize, usize, usize, usize) = (320, 274, 320, 202);

/// DeadRally's snapshot of the Arena, which the original has none of: the first circuit's
/// frame, its name's strip cleared (the menu writes the Arena's name in it), and its picture
/// a part of the Arena's track at its start line. Each of the track's colours becomes the
/// nearest of the colours the circuits' pictures use in the menu's palette, so that none of
/// them is one the menus change (the pulse, the copper, the player's colour).
pub fn arena_snapshot(
    snapshots: &[Image],
    menu: &Palette,
    (track, colours): (&Image, &Palette),
) -> Image {
    let Some(frame) = snapshots.first() else {
        return Image::new(0, 0, Vec::new());
    };
    let width = frame.width as usize;
    let mut pixels = frame.pixels.clone();
    let (strip_top, strip_lines) = NAME_STRIP;
    let (tile_x, tile) = STRIP_TILE;
    for y in strip_top..strip_top + strip_lines {
        for x in tile_x..width - tile_x {
            pixels[y * width + x] = frame.pixels[y * width + tile_x + (x - tile_x) % tile];
        }
    }
    let (left, top, picture_width, picture_lines) = PICTURE;
    let mut used = [false; 256];
    for circuit in snapshots.iter().take(CIRCUIT_SNAPSHOTS) {
        let stride = circuit.width as usize;
        for y in top..top + picture_lines {
            for x in left..left + picture_width {
                if let Some(&index) = circuit.pixels.get(y * stride + x) {
                    used[usize::from(index)] = true;
                }
            }
        }
    }
    // Each picture pixel the average of the view's pixels under it, in sixty-fourths, then
    // the nearest of the colours the pictures use.
    let nearest = |[r, g, b]: [u32; 3]| {
        let distance = |index: usize| {
            let [mr, mg, mb] = menu.0[index];
            let d = |a: u32, b: u8| (i64::from(a) - i64::from(b) * 64).pow(2);
            d(r, mr) + d(g, mg) + d(b, mb)
        };
        (0..256)
            .filter(|&index| used[index])
            .min_by_key(|&index| distance(index))
            .map_or(0, |index| index as u8)
    };
    let (view_x, view_y, view_width, view_lines) = ARENA_VIEW;
    let stride = track.width as usize;
    for y in 0..picture_lines {
        let (y0, y1) = (
            y * view_lines / picture_lines,
            (y + 1) * view_lines / picture_lines,
        );
        for x in 0..picture_width {
            let (x0, x1) = (
                x * view_width / picture_width,
                (x + 1) * view_width / picture_width,
            );
            let mut sum = [0u32; 3];
            let mut count = 0;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let index = track.pixels[(view_y + sy) * stride + view_x + sx];
                    for (total, part) in sum.iter_mut().zip(colours.0[usize::from(index)]) {
                        *total += u32::from(part);
                    }
                    count += 1;
                }
            }
            let average = sum.map(|total| total * 64 / count.max(1));
            pixels[(top + y) * width + left + x] = nearest(average);
        }
    }
    Image::new(frame.width, frame.height, pixels)
}

/// DeadRally's title font (docs/design/2026-10-13-personal-best-design.md): its 16 lines are
/// those of "TRACK RECORDS" (`RECOTXT`, drawn from line 92 of the menus' background), whose
/// letters it takes 1:1 and whose letters' parts make the others. Glyphs are a column apart, a
/// space 5 wide.
const TITLE_LINES: usize = 16;
const TITLE_GLYPHS: usize = 96;
const TITLE_SPACING: usize = 1;
const TITLE_SPACE: u8 = 5;
const RECORDS_TOP: usize = 92;
const RECORDS_TEXT: &[u8] = b"TRACKRECORDS";

/// A part of a made glyph, out of "TRACK RECORDS"'s letters, whose strokes are on lines 1 to
/// 14.
#[derive(Clone, Copy)]
enum Part {
    /// A letter's columns and lines, its first column set at a column of the glyph and the
    /// lines moved down (or up) some lines, each pixel moved then the colour of that line as
    /// bright beside the line's strokes.
    Copy(u8, (usize, usize), (usize, usize), isize, isize),
    /// K's stem on these lines at a column: its columns 0 to 4, and its glow (column 5) on
    /// its first and last lines, where its arms do not reach.
    Stem((usize, usize), isize),
    /// A slanted stroke: on each of these lines, K's stem across that line (its columns 1 to
    /// 4), from a column on the first line to one on the last, evenly.
    Slant((usize, usize), isize, isize),
    /// Nothing at these columns and lines.
    Clear((usize, usize), (usize, usize)),
}

use Part::{Clear, Slant, Stem};

/// A letter's columns and lines, set at a column of the glyph.
const fn copy(letter: u8, columns: (usize, usize), lines: (usize, usize), at: isize) -> Part {
    Part::Copy(letter, columns, lines, at, 0)
}

/// A whole letter.
const fn take(letter: u8) -> Part {
    copy(letter, (0, usize::MAX), (0, TITLE_LINES), 0)
}

/// K's foot as a dot, at a column.
const fn dot(at: isize) -> Part {
    copy(b'K', (0, 6), (11, 15), at)
}

/// The glyphs made of "TRACK RECORDS"'s letters' parts, the strokes as thick and bright as
/// theirs on every line (the owner's choice): P is R without its leg, B that with R's bowl
/// again below it, F and L are E's, I and the other stems K's, slanted strokes K's stem across each
/// line, the round digits O's halves, and so on. Each was checked by eye, enlarged, on a sheet
/// of them all.
const MADE: &[(u8, &[Part])] = &[
    (b'P', &[take(b'R'), Clear((4, 14), (9, 15))]),
    (
        b'B',
        &[
            copy(b'R', (0, 14), (1, 9), 0),
            copy(b'R', (0, 4), (9, 15), 0),
            Part::Copy(b'R', (4, 14), (2, 9), 4, 5),
            copy(b'D', (5, 11), (14, 15), 4),
        ],
    ),
    (b'F', &[take(b'E'), Clear((5, 14), (11, 15))]),
    (
        b'L',
        &[take(b'E'), Clear((0, 14), (1, 11)), Stem((1, 11), 0)],
    ),
    (b'I', &[Stem((1, 15), 0)]),
    (
        b'H',
        &[
            Stem((1, 15), 0),
            Stem((1, 15), 9),
            copy(b'E', (5, 9), (6, 9), 5),
        ],
    ),
    (
        b'N',
        &[Stem((1, 15), 0), Stem((1, 15), 10), Slant((1, 15), 1, 10)],
    ),
    (
        b'M',
        &[
            Stem((1, 15), 0),
            Stem((1, 15), 13),
            Slant((1, 11), 1, 7),
            Slant((1, 11), 13, 7),
        ],
    ),
    (b'V', &[Slant((1, 15), 0, 6), Slant((1, 15), 12, 6)]),
    (
        b'W',
        &[
            Slant((1, 15), 0, 4),
            Slant((1, 15), 9, 4),
            Slant((1, 15), 9, 14),
            Slant((1, 15), 18, 14),
        ],
    ),
    (b'X', &[Slant((1, 15), 0, 11), Slant((1, 15), 11, 0)]),
    (
        b'Y',
        &[
            Slant((1, 8), 0, 5),
            Slant((1, 8), 10, 5),
            copy(b'K', (1, 5), (8, 15), 5),
        ],
    ),
    (
        b'Z',
        &[
            copy(b'T', (0, 15), (1, 4), 0),
            Clear((6, 10), (3, 4)),
            copy(b'T', (2, 6), (3, 4), 6),
            Slant((4, 12), 10, 1),
            copy(b'E', (1, 8), (12, 15), 0),
            copy(b'E', (5, 14), (12, 15), 6),
        ],
    ),
    (
        b'U',
        &[
            copy(b'O', (0, 16), (5, 15), 0),
            copy(b'K', (0, 6), (1, 5), -1),
            copy(b'K', (0, 6), (1, 5), 11),
        ],
    ),
    (
        b'G',
        &[
            take(b'C'),
            copy(b'E', (5, 11), (7, 9), 7),
            copy(b'K', (1, 5), (8, 12), 10),
        ],
    ),
    (b'Q', &[take(b'O'), copy(b'K', (7, 15), (11, 15), 9)]),
    (
        b'J',
        &[
            copy(b'O', (0, 8), (10, 15), 0),
            copy(b'O', (10, 16), (10, 15), 8),
            Stem((1, 10), 9),
        ],
    ),
    (
        b'0',
        &[
            copy(b'O', (0, 8), (1, 15), 0),
            copy(b'O', (10, 16), (1, 15), 8),
        ],
    ),
    (b'1', &[Stem((1, 15), 3), Slant((2, 5), 2, 0)]),
    (
        b'2',
        &[
            copy(b'O', (0, 16), (1, 4), 0),
            copy(b'O', (9, 16), (4, 8), 9),
            Slant((7, 12), 11, 1),
            copy(b'E', (0, 14), (12, 15), 0),
        ],
    ),
    (
        b'3',
        &[
            take(b'R'),
            Clear((4, 14), (9, 15)),
            Clear((0, 4), (9, 15)),
            copy(b'D', (0, 16), (9, 15), -1),
            Clear((0, 4), (4, 12)),
            Clear((4, 6), (6, 10)),
        ],
    ),
    (
        b'4',
        &[
            Slant((1, 10), 7, 0),
            Stem((1, 15), 8),
            copy(b'A', (5, 10), (9, 12), 0),
            copy(b'A', (5, 10), (9, 12), 5),
            copy(b'A', (5, 10), (9, 12), 9),
        ],
    ),
    (
        b'5',
        &[
            copy(b'E', (0, 14), (1, 4), 0),
            copy(b'E', (0, 5), (4, 6), 0),
            copy(b'S', (0, 14), (6, 15), 0),
        ],
    ),
    (
        b'6',
        &[
            copy(b'C', (0, 7), (1, 6), 0),
            copy(b'C', (8, 15), (1, 6), 7),
            copy(b'O', (0, 7), (6, 10), 0),
            Part::Copy(b'O', (0, 7), (2, 4), 0, 6),
            Part::Copy(b'O', (9, 16), (2, 4), 7, 6),
            copy(b'O', (0, 7), (10, 15), 0),
            copy(b'O', (9, 16), (10, 15), 7),
        ],
    ),
    (
        b'7',
        &[copy(b'E', (1, 14), (1, 4), 0), Slant((4, 15), 9, 2)],
    ),
    (
        b'8',
        &[
            copy(b'O', (0, 7), (1, 6), 0),
            copy(b'O', (9, 16), (1, 6), 7),
            Part::Copy(b'O', (0, 7), (11, 14), 0, -5),
            Part::Copy(b'O', (9, 16), (11, 14), 7, -5),
            Part::Copy(b'O', (0, 7), (2, 4), 0, 6),
            Part::Copy(b'O', (9, 16), (2, 4), 7, 6),
            copy(b'O', (0, 7), (10, 15), 0),
            copy(b'O', (9, 16), (10, 15), 7),
        ],
    ),
    (
        b'9',
        &[
            copy(b'O', (0, 7), (1, 6), 0),
            copy(b'O', (9, 16), (1, 6), 7),
            Part::Copy(b'O', (0, 7), (11, 14), 0, -5),
            Part::Copy(b'O', (9, 16), (11, 14), 7, -5),
            copy(b'O', (9, 16), (6, 10), 7),
            copy(b'S', (0, 14), (10, 15), 0),
        ],
    ),
    (b'.', &[dot(0)]),
    (
        b',',
        &[
            copy(b'K', (1, 5), (10, 14), 1),
            copy(b'K', (0, 4), (14, 15), 0),
        ],
    ),
    (b':', &[copy(b'K', (0, 5), (4, 8), 0), dot(0)]),
    (
        b';',
        &[
            copy(b'K', (0, 5), (4, 8), 1),
            copy(b'K', (1, 5), (10, 14), 2),
            copy(b'K', (0, 4), (14, 15), 1),
        ],
    ),
    (
        b'-',
        &[
            copy(b'E', (5, 11), (6, 9), 0),
            copy(b'E', (5, 11), (6, 9), 3),
        ],
    ),
    (b'!', &[Stem((1, 10), 0), dot(0)]),
    (
        b'?',
        &[
            copy(b'O', (4, 16), (1, 4), 0),
            copy(b'O', (10, 16), (4, 7), 6),
            Slant((7, 10), 6, 5),
            dot(5),
        ],
    ),
    (b'\'', &[Stem((1, 6), 0)]),
    (b'"', &[Stem((1, 6), 0), Stem((1, 6), 6)]),
    (b'`', &[Slant((1, 5), 0, 2)]),
    (b'/', &[Slant((1, 15), 8, 0)]),
    (b'\\', &[Slant((1, 15), 0, 8)]),
    (b'|', &[Stem((1, 15), 0)]),
    (
        b'(',
        &[
            Slant((1, 5), 3, 1),
            Slant((5, 11), 0, 0),
            Slant((11, 15), 1, 3),
        ],
    ),
    (
        b')',
        &[
            Slant((1, 5), 0, 2),
            Slant((5, 11), 3, 3),
            Slant((11, 15), 2, 0),
        ],
    ),
    (
        b'[',
        &[
            Stem((1, 15), 0),
            copy(b'E', (5, 9), (1, 4), 5),
            copy(b'E', (5, 9), (12, 15), 5),
        ],
    ),
    (
        b']',
        &[
            copy(b'E', (5, 9), (1, 4), 0),
            copy(b'E', (5, 9), (12, 15), 0),
            Stem((1, 15), 3),
        ],
    ),
    (b'<', &[Slant((3, 8), 6, 0), Slant((8, 13), 0, 6)]),
    (b'>', &[Slant((3, 8), 0, 6), Slant((8, 13), 6, 0)]),
    (
        b'+',
        &[
            Stem((4, 12), 4),
            copy(b'E', (5, 11), (6, 9), 0),
            copy(b'E', (5, 11), (6, 9), 6),
        ],
    ),
    (
        b'=',
        &[
            Part::Copy(b'E', (5, 11), (6, 9), 0, -2),
            Part::Copy(b'E', (5, 11), (6, 9), 5, -2),
            copy(b'A', (4, 12), (9, 12), 0),
            copy(b'A', (4, 12), (9, 12), 3),
        ],
    ),
    (b'_', &[copy(b'E', (1, 14), (12, 15), 0)]),
    (
        b'*',
        &[Stem((1, 8), 4), Slant((2, 7), 0, 8), Slant((2, 7), 8, 0)],
    ),
    (b'^', &[Slant((2, 7), 3, 0), Slant((2, 7), 3, 6)]),
    (
        b'~',
        &[
            copy(b'E', (5, 9), (6, 9), 0),
            Part::Copy(b'E', (5, 9), (6, 9), 3, -1),
            copy(b'E', (5, 9), (6, 9), 6),
            Part::Copy(b'E', (5, 9), (6, 9), 9, 1),
            copy(b'E', (5, 9), (6, 9), 12),
        ],
    ),
    (
        b'#',
        &[
            Stem((1, 15), 2),
            Stem((1, 15), 9),
            Part::Copy(b'E', (5, 10), (6, 9), 0, -2),
            Part::Copy(b'E', (5, 10), (6, 9), 5, -2),
            Part::Copy(b'E', (5, 11), (6, 9), 9, -2),
            copy(b'A', (4, 12), (9, 12), 0),
            copy(b'A', (4, 12), (9, 12), 7),
        ],
    ),
    (b'$', &[take(b'S'), copy(b'K', (2, 4), (1, 15), 6)]),
    (
        b'%',
        &[
            Slant((1, 15), 12, 2),
            copy(b'K', (0, 6), (1, 5), 0),
            dot(10),
        ],
    ),
    (
        b'&',
        &[
            copy(b'O', (0, 6), (1, 6), 0),
            copy(b'O', (11, 16), (1, 6), 6),
            Part::Copy(b'O', (0, 6), (11, 14), 0, -5),
            Part::Copy(b'O', (11, 16), (11, 14), 6, -5),
            Part::Copy(b'O', (0, 7), (2, 4), 0, 6),
            copy(b'O', (0, 7), (10, 15), 0),
            copy(b'O', (7, 10), (12, 15), 7),
            Slant((7, 15), 4, 11),
            Slant((8, 12), 14, 11),
        ],
    ),
    (
        b'@',
        &[
            take(b'O'),
            Part::Copy(b'K', (1, 5), (11, 15), 6, -5),
            copy(b'K', (2, 4), (5, 12), 9),
        ],
    ),
    (
        b'{',
        &[
            Slant((1, 6), 4, 2),
            Slant((6, 8), 2, 0),
            Slant((8, 10), 0, 2),
            Slant((10, 15), 2, 4),
        ],
    ),
    (
        b'}',
        &[
            Slant((1, 6), 0, 2),
            Slant((6, 8), 2, 4),
            Slant((8, 10), 4, 2),
            Slant((10, 15), 2, 0),
        ],
    ),
];

/// DeadRally's font for titles: every printable character, as "TRACK RECORDS" is lettered.
/// Its letters are taken 1:1: the first of each, the pixels where `RECOTXT` differs from
/// the menus' background under it, letters split where nothing of them is between or where
/// least is (its A and C touch). The others are made of their parts ([`MADE`]), small
/// letters as capitals. The glyphs, and the pen's advance after each.
pub fn title_font(title: &Image, background: &Image, palette: &Palette) -> (Vec<Image>, Vec<u8>) {
    let width = title.width as usize;
    let lines = (title.height as usize).min(TITLE_LINES);
    let letter = |x: usize, y: usize| {
        let pixel = title.pixels[y * width + x];
        (pixel != background.pixels[(RECORDS_TOP + y) * background.width as usize + x])
            .then_some(pixel)
    };
    let brightness = |index: u8| {
        palette.0[usize::from(index)]
            .iter()
            .map(|&part| u32::from(part))
            .sum::<u32>()
    };
    // Each line's colours, and its strokes' (its brightest).
    let mut colours: Vec<Vec<u8>> = vec![Vec::new(); TITLE_LINES];
    let mut strokes: Vec<Option<u8>> = vec![None; TITLE_LINES];
    for y in 0..lines {
        for x in 0..width {
            if let Some(pixel) = letter(x, y) {
                if !colours[y].contains(&pixel) {
                    colours[y].push(pixel);
                }
                if strokes[y].is_none_or(|best| brightness(pixel) > brightness(best)) {
                    strokes[y] = Some(pixel);
                }
            }
        }
    }
    let index = |c: u8| usize::from(c - 32);
    let mut glyphs = vec![Image::new(0, TITLE_LINES as u32, Vec::new()); TITLE_GLYPHS];
    let mut advances = vec![TITLE_SPACE; TITLE_GLYPHS];
    // "TRACK RECORDS"'s own letters, split where nothing of them is between; a span as wide
    // as two letters is two, split at its thinnest column.
    let inked = |x: usize| (0..lines).filter(|&y| letter(x, y).is_some()).count();
    let mut spans = Vec::new();
    let mut x = 0;
    while x < width {
        if inked(x) == 0 {
            x += 1;
            continue;
        }
        let start = x;
        while x < width && inked(x) > 0 {
            x += 1;
        }
        spans.push((start, x));
    }
    let widest = 20;
    let mut letters = Vec::new();
    for (start, end) in spans {
        if end - start > widest {
            let split = (start + 8..end - 8)
                .min_by_key(|&x| inked(x))
                .unwrap_or(start + 8);
            letters.push((start, split + 1));
            letters.push((split + 1, end));
        } else {
            letters.push((start, end));
        }
    }
    let mut taken = [false; TITLE_GLYPHS];
    for (&c, &(start, end)) in RECORDS_TEXT.iter().zip(&letters) {
        let at = index(c);
        if std::mem::replace(&mut taken[at], true) {
            continue;
        }
        let glyph_width = end - start;
        let mut pixels = vec![0; glyph_width * TITLE_LINES];
        for y in 0..lines {
            for x in start..end {
                pixels[y * glyph_width + x - start] = letter(x, y).unwrap_or(0);
            }
        }
        glyphs[at] = Image::new(glyph_width as u32, TITLE_LINES as u32, pixels);
        advances[at] = (glyph_width + TITLE_SPACING) as u8;
    }
    // A pixel moved from line `from` to line `to`: the colour of `to` nearest to its
    // strokes' as much brighter or darker as the pixel is beside `from`'s.
    let recolour = |pixel: u8, from: usize, to: usize| {
        let (Some(was), Some(stroke)) = (strokes[from], strokes[to]) else {
            return pixel;
        };
        let (part, whole) = (i64::from(brightness(pixel)), i64::from(brightness(was)));
        let stroke = palette.0[usize::from(stroke)];
        colours[to]
            .iter()
            .copied()
            .min_by_key(|&colour| {
                palette.0[usize::from(colour)]
                    .iter()
                    .zip(stroke)
                    .map(|(&a, b)| (i64::from(a) * whole - i64::from(b) * part).pow(2))
                    .sum::<i64>()
            })
            .unwrap_or(pixel)
    };
    for &(c, parts) in MADE {
        let mut columns: Vec<[u8; TITLE_LINES]> = Vec::new();
        // Where parts cross, the brighter pixel shows.
        let put = |columns: &mut Vec<[u8; TITLE_LINES]>, x: isize, y: usize, pixel: u8| {
            let Ok(x) = usize::try_from(x) else {
                return;
            };
            if pixel == 0 || y >= TITLE_LINES {
                return;
            }
            if columns.len() <= x {
                columns.resize(x + 1, [0; TITLE_LINES]);
            }
            let old = columns[x][y];
            if old == 0 || brightness(pixel) > brightness(old) {
                columns[x][y] = pixel;
            }
        };
        let at = |c: u8, x: usize, y: usize| {
            let glyph = &glyphs[index(c)];
            glyph.pixels[y * glyph.width as usize + x]
        };
        for &part in parts {
            match part {
                Part::Copy(from, (x0, x1), (y0, y1), to, down) => {
                    let x1 = x1.min(glyphs[index(from)].width as usize);
                    for y in y0..y1 {
                        let Some(line) = y.checked_add_signed(down) else {
                            continue;
                        };
                        for x in x0..x1 {
                            let pixel = at(from, x, y);
                            let pixel = if line == y || pixel == 0 || line >= TITLE_LINES {
                                pixel
                            } else {
                                recolour(pixel, y, line)
                            };
                            put(&mut columns, to + (x - x0) as isize, line, pixel);
                        }
                    }
                }
                Stem((y0, y1), to) => {
                    for y in y0..y1 {
                        let glow = if y == 1 || y == 14 { 6 } else { 5 };
                        for x in 0..glow {
                            put(&mut columns, to + x as isize, y, at(b'K', x, y));
                        }
                    }
                }
                Slant((y0, y1), from, to) => {
                    let steps = (y1 - 1 - y0).max(1) as isize;
                    for y in y0..y1 {
                        let along = halves_to_even((to - from) * (y - y0) as isize, steps);
                        for x in 1..5 {
                            put(
                                &mut columns,
                                from + along + x as isize - 1,
                                y,
                                at(b'K', x, y),
                            );
                        }
                    }
                }
                Clear((x0, x1), (y0, y1)) => {
                    for column in columns.iter_mut().take(x1).skip(x0) {
                        column[y0..y1].fill(0);
                    }
                }
            }
        }
        while columns
            .last()
            .is_some_and(|column| column.iter().all(|&p| p == 0))
        {
            columns.pop();
        }
        let glyph_width = columns.len();
        let pixels = (0..TITLE_LINES)
            .flat_map(|y| columns.iter().map(move |column| column[y]))
            .collect();
        glyphs[index(c)] = Image::new(glyph_width as u32, TITLE_LINES as u32, pixels);
        advances[index(c)] = (glyph_width + TITLE_SPACING) as u8;
    }
    for c in b'a'..=b'z' {
        let capital = index(c.to_ascii_uppercase());
        glyphs[index(c)] = glyphs[capital].clone();
        advances[index(c)] = advances[capital];
    }
    (glyphs, advances)
}

/// `part` / `whole` rounded to the nearest whole, halves to the even one.
fn halves_to_even(part: isize, whole: isize) -> isize {
    let (quotient, rest) = (part.div_euclid(whole), part.rem_euclid(whole));
    match (2 * rest).cmp(&whole) {
        std::cmp::Ordering::Less => quotient,
        std::cmp::Ordering::Equal => quotient + (quotient & 1),
        std::cmp::Ordering::Greater => quotient + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_end_screen_of_another_size_is_an_error_not_a_crash_later() {
        // The menu copies END.BMP over its whole 640x480 screen; a BMP of another size (from
        // an unknown release) is reported when the data loads, naming the file.
        let path = std::path::Path::new("END.BMP");
        let screen = |width: u32, height: u32| Picture {
            image: Image::new(width, height, vec![0; (width * height) as usize]),
            palette: Palette::BLACK,
        };
        assert!(full_screen(screen(640, 480), path).is_ok());
        let error = full_screen(screen(320, 200), path).unwrap_err();
        assert_eq!(
            error.to_string(),
            "END.BMP: unsupported BMP: 320x200, not 640x480"
        );
    }
}
