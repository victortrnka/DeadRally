//! The decoded data the startup sequence needs (spec M1a §4.2, M1b §4.1).

use std::fmt;
use std::path::PathBuf;

use crate::bmp::{self, BmpError};
use crate::bpa::{Archive, BpaError};
use crate::catalog::{self, CatalogError};
use crate::haf::{Animation, HafError};
use crate::image::{Image, Palette, PaletteError};
use crate::s3m::Module;
use crate::sound::{self, SoundError};
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
        })
    }
}

fn picture(
    menu: &Archive,
    image: &'static str,
    palette: &'static str,
) -> Result<Picture, AssetError> {
    let entry = catalog::find("MENU.BPA", image).expect("startup images are catalogued");
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
