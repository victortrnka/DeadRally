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
    pub menu: MenuAssets,
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
        let remedy_path = path("RMD.BMP");
        let remedy_bytes = std::fs::read(&remedy_path).map_err(|source| AssetError::Read {
            path: remedy_path.clone(),
            source,
        })?;
        let (image, palette) = bmp::decode(&remedy_bytes).map_err(|error| AssetError::Bmp {
            path: remedy_path,
            error,
        })?;
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
            menu: menu_assets(&menu, &musics, &path("END.BMP"), &path("DR.EXE"))?,
        })
    }
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
    exe: &std::path::Path,
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
    let exe_bytes = std::fs::read(exe).map_err(|source| AssetError::Read {
        path: exe.to_path_buf(),
        source,
    })?;
    let exe = Exe::parse(exe_bytes).map_err(AssetError::Exe)?;
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
        background_copper: background_copper.as_chunks::<3>().0.to_vec(),
        credits: vec![
            picture(menu, "CREDIT1.BPK", "CREDIT1.PAL")?,
            picture(menu, "CREDIT2.BPK", "CREDIT2.PAL")?,
        ],
        end: full_screen(bmp_picture(end)?, end)?,
        effects: sound::load_effects(musics, "MEN-SAM.CMF").map_err(AssetError::Sound)?,
        texts: Texts::read(&exe).map_err(AssetError::Text)?,
        slider: frames(menu, "SLIDMUS2.BPK")?.remove(0),
        knob: frames(menu, "VOLCUR2.BPK")?.remove(0),
        default_config: DrCfg::defaults(&exe).map_err(AssetError::Machine)?,
        medium: frames(menu, "F-MED1A.BPK")?,
        fame_title: frames(menu, "FAMETXT.BPK")?.remove(0),
        records_title: frames(menu, "RECOTXT.BPK")?.remove(0),
        records_bar: frames(menu, "RECOBAR.BPK")?.remove(0),
        snapshots: frames(menu, "TRSNAP2M.BPK")?,
        arrows: frames(menu, "TRARR1.BPK")?,
        wipe: frames(menu, "15X150.BPK")?,
        border_corners: frames(menu, "CHOO2.BPK")?,
        licence: frames(menu, "LICENCE3.BPK")?.remove(0),
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
        market_prices: market_prices(&exe).map_err(AssetError::Machine)?,
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
