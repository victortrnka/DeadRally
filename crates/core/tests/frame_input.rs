//! The frame and input types frontends build on (spec section 5).

use deadrally_core::{Frame, Key, PadButton, expand_6bit};

#[test]
fn six_bit_palette_values_expand_like_the_windows_version() {
    // The Windows version shifts by two, so its white is 252; reference screenshots of the
    // original only match pixel for pixel with the same expansion.
    assert_eq!(expand_6bit(0), 0);
    assert_eq!(expand_6bit(63), 252);
    assert_eq!(expand_6bit(32), 128);
    // The VGA DAC ignores the top two bits.
    assert_eq!(expand_6bit(0x40 | 63), 252);
}

#[test]
fn write_rgba_uses_the_palette_for_every_pixel() {
    let mut palette = [[0u8; 3]; 256];
    palette[1] = [63, 0, 0];
    palette[2] = [0, 63, 32];
    let pixels = [1, 2, 0];
    let frame = Frame {
        width: 3,
        height: 1,
        pixels: &pixels,
        palette: &palette,
        aspect: (4, 3),
    };
    let mut rgba = [0u8; 12];
    frame.write_rgba(&mut rgba);
    assert_eq!(rgba, [252, 0, 0, 255, 0, 252, 128, 255, 0, 0, 0, 255]);
}

#[test]
fn key_all_lists_every_key_in_declaration_order() {
    // The test scene indexes its key grid with `key as usize`.
    assert_eq!(Key::ALL.len(), 79);
    for (index, &key) in Key::ALL.iter().enumerate() {
        assert_eq!(key as usize, index);
    }
    for (index, &button) in PadButton::ALL.iter().enumerate() {
        assert_eq!(button as usize, index);
    }
}
