//! A run through the main menu against the developer's real game data. Run with
//! `cargo test-data`; the test reads DEADRALLY_DATA and fails (never passes silently) when it is
//! unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use sha2::{Digest, Sha256};

/// The keys of `scripts/reference/menu-keys.scenario`, at the ticks where the original read
/// them in the run its screenshots come from (docs/verification/m2a.md): the highlight up and
/// down, Escape, the start submenu, the exit question, the credits and the end screen.
const KEYS: [(u64, Key); 30] = [
    (6423, Key::Down),
    (6495, Key::Down),
    (6566, Key::Up),
    (6638, Key::Up),
    (6709, Key::Down),
    (6780, Key::Escape),
    (6852, Key::Up),
    (6923, Key::Up),
    (6995, Key::Up),
    (7066, Key::Up),
    (7137, Key::Enter),
    (7209, Key::Down),
    (7280, Key::Down),
    (7352, Key::Enter),
    (7423, Key::Enter),
    (7495, Key::Escape),
    (7566, Key::Escape),
    (7637, Key::Enter),
    (7709, Key::Left),
    (7780, Key::Right),
    (7852, Key::Escape),
    (7923, Key::Up),
    (7995, Key::Enter),
    (8138, Key::Space),
    (8280, Key::Space),
    (8423, Key::Down),
    (8495, Key::Enter),
    (8566, Key::Left),
    (8638, Key::Enter),
    (8923, Key::Space),
];

/// The frame after each of these ticks equals the scenario's screenshot of that name.
const SHOTS: [(u64, &str); 47] = [
    (6388, "idle"),
    (6459, "after-90000"),
    (6530, "after-91000"),
    (6602, "after-92000"),
    (6673, "after-93000"),
    (6745, "after-94000"),
    (6816, "after-95000"),
    (6888, "after-96000"),
    (6959, "after-97000"),
    (7030, "after-98000"),
    (7102, "after-99000"),
    (7173, "after-100000"),
    (7245, "after-101000"),
    (7316, "after-102000"),
    (7387, "after-103000"),
    (7459, "after-104000"),
    (7530, "after-105000"),
    (7602, "after-106000"),
    (7673, "after-107000"),
    (7745, "after-108000"),
    (7816, "after-109000"),
    (7887, "after-110000"),
    (7959, "after-111000"),
    (8009, "credits-112200"),
    (8023, "credits-112400"),
    (8045, "credits-112700"),
    (8066, "credits-113000"),
    (8101, "credits-113500"),
    (8151, "credits-114200"),
    (8172, "credits-114500"),
    (8194, "credits-114800"),
    (8223, "credits-115200"),
    (8294, "credits-116200"),
    (8315, "credits-116500"),
    (8344, "credits-116900"),
    (8387, "credits-117500"),
    (8458, "after-118000"),
    (8530, "after-119000"),
    (8601, "after-120000"),
    (8651, "end-121200"),
    (8672, "end-121500"),
    (8694, "end-121800"),
    (8723, "end-122200"),
    (8780, "end-123000"),
    (8851, "end-124000"),
    (8929, "end-125100"),
    (8937, "end-125200"),
];

/// The menu run ends when the game asks to quit, a little after its last screenshot.
const MAX_TICKS: u64 = 9_000;

/// The keys of `scripts/reference/menu-configure.scenario` in the run of
/// docs/verification/m2b.md: both volume popups, Define Keyboard with Q for accelerate, Define
/// Gamepad, the gamepad switch (no gamepad under Wine), Escape and "previous menu".
const CONFIGURE_KEYS: [(u64, Key); 24] = [
    (6420, Key::Down),
    (6491, Key::Enter),
    (6563, Key::Enter),
    (6634, Key::Left),
    (6656, Key::Left),
    (6677, Key::Left),
    (6742, Key::Enter),
    (6813, Key::Down),
    (6849, Key::Enter),
    (6920, Key::Right),
    (6992, Key::Enter),
    (7027, Key::Down),
    (7063, Key::Enter),
    (7134, Key::Enter),
    (7206, Key::Q),
    (7277, Key::Escape),
    (7349, Key::Down),
    (7385, Key::Enter),
    (7456, Key::Escape),
    (7528, Key::Down),
    (7563, Key::Enter),
    (7635, Key::Space),
    (7706, Key::Escape),
    (7778, Key::Enter),
];

const CONFIGURE_SHOTS: [(u64, &str); 17] = [
    (6384, "idle"),
    (6527, "configure"),
    (6598, "music"),
    (6705, "music-left"),
    (6777, "after-music"),
    (6884, "effects"),
    (6955, "effects-right"),
    (7098, "keyboard"),
    (7170, "press-key"),
    (7241, "after-q"),
    (7312, "after-keyboard"),
    (7419, "gamepad"),
    (7491, "after-gamepad"),
    (7598, "not-detected"),
    (7670, "after-not-detected"),
    (7741, "escape"),
    (7812, "previous"),
];

/// The keys of `scripts/reference/menu-hall-of-fame.scenario` in the run of
/// docs/verification/m2c.md: into the Hall of Fame, on to the records, Right, Left twice and
/// Escape.
const HALL_OF_FAME_KEYS: [(u64, Key); 8] = [
    (6455, Key::Down),
    (6491, Key::Down),
    (6527, Key::Enter),
    (6814, Key::Space),
    (7027, Key::Right),
    (7134, Key::Left),
    (7205, Key::Left),
    (7314, Key::Escape),
];

const HALL_OF_FAME_SHOTS: [(u64, &str); 78] = [
    (6420, "idle"),
    (6534, "fame-91100"),
    (6541, "fame-91200"),
    (6548, "fame-91300"),
    (6556, "fame-91400"),
    (6563, "fame-91500"),
    (6570, "fame-91600"),
    (6577, "fame-91700"),
    (6584, "fame-91800"),
    (6591, "fame-91900"),
    (6598, "fame-92000"),
    (6606, "fame-92100"),
    (6613, "fame-92200"),
    (6620, "fame-92300"),
    (6627, "fame-92400"),
    (6634, "fame-92500"),
    (6641, "fame-92600"),
    (6648, "fame-92700"),
    (6655, "fame-92800"),
    (6663, "fame-92900"),
    (6741, "fame"),
    (6820, "records-95100"),
    (6827, "records-95200"),
    (6834, "records-95300"),
    (6841, "records-95400"),
    (6848, "records-95500"),
    (6856, "records-95600"),
    (6863, "records-95700"),
    (6870, "records-95800"),
    (6877, "records-95900"),
    (6884, "records-96000"),
    (6891, "records-96100"),
    (6898, "records-96200"),
    (6906, "records-96300"),
    (6913, "records-96400"),
    (6920, "records-96500"),
    (6927, "records-96600"),
    (6934, "records-96700"),
    (6941, "records-96800"),
    (6948, "records-96900"),
    (6991, "records"),
    (7031, "right-98050"),
    (7034, "right-98100"),
    (7038, "right-98150"),
    (7041, "right-98200"),
    (7045, "right-98250"),
    (7049, "right-98300"),
    (7052, "right-98350"),
    (7099, "after-right"),
    (7170, "after-left"),
    (7209, "left-100550"),
    (7213, "left-100600"),
    (7216, "left-100650"),
    (7220, "left-100700"),
    (7224, "left-100750"),
    (7227, "left-100800"),
    (7231, "left-100850"),
    (7277, "after-left-2"),
    (7320, "back-102100"),
    (7327, "back-102200"),
    (7334, "back-102300"),
    (7341, "back-102400"),
    (7349, "back-102500"),
    (7356, "back-102600"),
    (7363, "back-102700"),
    (7370, "back-102800"),
    (7377, "back-102900"),
    (7384, "back-103000"),
    (7392, "back-103100"),
    (7399, "back-103200"),
    (7406, "back-103300"),
    (7413, "back-103400"),
    (7420, "back-103500"),
    (7427, "back-103600"),
    (7434, "back-103700"),
    (7442, "back-103800"),
    (7449, "back-103900"),
    (7527, "menu-back"),
];

/// The keys of `scripts/reference/new-game.scenario` in the run of
/// `docs/verification/m3a.md`: the intro skipped, a licence filled in, the sign-up's welcome,
/// the medium race's warning and a sign-up for the easy race.
const NEW_GAME_KEYS: [(u64, Key); 22] = [
    (130, Key::Space),
    (1727, Key::Enter),
    (1799, Key::Enter),
    (1885, Key::A),
    (1906, Key::B),
    (1927, Key::C),
    (1985, Key::Backspace),
    (2042, Key::C),
    (2077, Key::Down),
    (2160, Key::Right),
    (2175, Key::Right),
    (2189, Key::Right),
    (2239, Key::Enter),
    (2325, Key::Enter),
    (2410, Key::Down),
    (2482, Key::Enter),
    (2660, Key::Enter),
    (2746, Key::Right),
    (2817, Key::Enter),
    (2903, Key::Space),
    (2975, Key::Left),
    (3046, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run.
const NEW_GAME_SHOTS: [(u64, &str); 36] = [
    (1691, "idle"),
    (1848, "licence"),
    (1949, "typed"),
    (2012, "erased"),
    (2080, "face-early"),
    (2102, "face-lit"),
    (2130, "face"),
    (2202, "colour"),
    (2288, "weapons"),
    (2374, "difficulty"),
    (2444, "medium"),
    (2488, "wipe-01"),
    (2495, "wipe-02"),
    (2502, "wipe-03"),
    (2509, "wipe-04"),
    (2516, "wipe-05"),
    (2524, "wipe-06"),
    (2531, "wipe-07"),
    (2537, "wipe-08"),
    (2545, "wipe-09"),
    (2551, "wipe-10"),
    (2559, "wipe-11"),
    (2565, "wipe-12"),
    (2609, "welcome"),
    (2668, "sign-up"),
    (2747, "right"),
    (2825, "warning"),
    (2938, "warned"),
    (3007, "left"),
    (3052, "signed"),
    (3081, "filling-0"),
    (3117, "filling-1"),
    (3152, "filling-2"),
    (3154, "filling-3"),
    (3224, "filling-4"),
    (3225, "filling-5"),
];

/// The keys of `scripts/reference/saved-games.scenario` in the run of
/// `docs/verification/m3b.md`: the intro skipped, a saved game loaded, the shop's items and
/// car arrows, back to the menu, and a save into slot 1 named "xyz".
const SAVED_GAMES_KEYS: [(u64, Key); 26] = [
    (130, Key::Space),
    (1727, Key::Enter),
    (1813, Key::Down),
    (1885, Key::Enter),
    (1970, Key::Enter),
    (2062, Key::Space),
    (2190, Key::Left),
    (2269, Key::Left),
    (2348, Key::Left),
    (2426, Key::Left),
    (2504, Key::Left),
    (2583, Key::Up),
    (2661, Key::Right),
    (2740, Key::Left),
    (2818, Key::Down),
    (2898, Key::Right),
    (2975, Key::Escape),
    (3090, Key::Down),
    (3161, Key::Enter),
    (3247, Key::Down),
    (3318, Key::Enter),
    (3404, Key::X),
    (3426, Key::Y),
    (3447, Key::Z),
    (3497, Key::Enter),
    (3583, Key::Space),
];

/// The ticks after which our frame equalled each screenshot of that run.
const SAVED_GAMES_SHOTS: [(u64, &str); 31] = [
    (1689, "idle"),
    (1783, "start"),
    (1855, "load-row"),
    (1941, "slots"),
    (2026, "loaded"),
    (2068, "shop-wipe-1"),
    (2075, "shop-wipe-2"),
    (2082, "shop-wipe-3"),
    (2090, "shop-wipe-4"),
    (2097, "shop-wipe-5"),
    (2104, "shop-wipe-6"),
    (2111, "shop-wipe-7"),
    (2154, "shop"),
    (2240, "left-1"),
    (2318, "left-2"),
    (2397, "left-3"),
    (2475, "left-4"),
    (2554, "left-5"),
    (2632, "up"),
    (2711, "car-next"),
    (2790, "car-back"),
    (2868, "down"),
    (2947, "right"),
    (3059, "menu"),
    (3129, "save-row"),
    (3216, "save-slots"),
    (3287, "slot-1"),
    (3319, "prompt"),
    (3448, "named"),
    (3498, "saved"),
    (3640, "after"),
];

/// The keys of `scripts/reference/shop-purchases.scenario` in the run of
/// `docs/verification/m3c.md`: the test game loaded, every upgrade, the repairs, a car too
/// dear, and a cheaper one declined, bought and painted.
const PURCHASES_KEYS: [(u64, Key); 44] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2035, Key::Space),
    (2163, Key::Left),
    (2221, Key::Left),
    (2277, Key::Left),
    (2335, Key::Left),
    (2392, Key::Enter),
    (2449, Key::Right),
    (2506, Key::Enter),
    (2563, Key::Right),
    (2620, Key::Enter),
    (2678, Key::Enter),
    (2735, Key::Right),
    (2792, Key::Enter),
    (2849, Key::Enter),
    (2906, Key::Enter),
    (2963, Key::Enter),
    (3020, Key::Enter),
    (3742, Key::Left),
    (3806, Key::Left),
    (3870, Key::Left),
    (3935, Key::Up),
    (3999, Key::Right),
    (4063, Key::Right),
    (4128, Key::Right),
    (4192, Key::Right),
    (4256, Key::Enter),
    (4320, Key::Left),
    (4385, Key::Left),
    (4449, Key::Enter),
    (4513, Key::Right),
    (4577, Key::Enter),
    (4642, Key::Enter),
    (4706, Key::Enter),
    (4770, Key::Right),
    (4835, Key::Right),
    (4899, Key::Right),
    (4963, Key::Right),
    (5027, Key::Right),
    (5092, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run.
const PURCHASES_SHOTS: [(u64, &str); 46] = [
    (1691, "idle"),
    (1785, "k01-return"),
    (1849, "k02-down"),
    (1927, "k03-return"),
    (1956, "k04-return"),
    (2141, "k05-space"),
    (2198, "k06-left"),
    (2255, "k07-left"),
    (2313, "k08-left"),
    (2370, "k09-left"),
    (2393, "k10-return"),
    (2484, "k11-right"),
    (2523, "k12-return"),
    (2598, "k13-right"),
    (2655, "k14-return"),
    (2679, "k15-return"),
    (2770, "k16-right"),
    (2827, "k17-return"),
    (2884, "k18-return"),
    (2941, "k19-return"),
    (2998, "k20-return"),
    (3055, "k21-return"),
    (3720, "messages-gone"),
    (3784, "k22-left"),
    (3814, "k23-left"),
    (3871, "k24-left"),
    (3977, "k25-up"),
    (4041, "k26-right"),
    (4106, "k27-right"),
    (4170, "k28-right"),
    (4234, "k29-right"),
    (4298, "k30-return"),
    (4363, "k31-left"),
    (4427, "k32-left"),
    (4491, "k33-return"),
    (4556, "k34-right"),
    (4620, "k35-return"),
    (4684, "k36-return"),
    (4748, "k37-return"),
    (4813, "k38-right"),
    (4877, "k39-right"),
    (4941, "k40-right"),
    (5006, "k41-right"),
    (5070, "k42-right"),
    (5134, "k43-return"),
    (5263, "car-bought-later"),
];

/// The keys of `scripts/reference/market.scenario` in the run of `docs/verification/m3c.md`:
/// the test game loaded, the Underground Market from the shop, the loan shark's loan and its
/// paying back, every weapon, Escape back to the shop, the market again and on to the sign-up.
const MARKET_KEYS: [(u64, Key); 27] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1948, Key::Enter),
    (2032, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Left),
    (2479, Key::Left),
    (2543, Key::Left),
    (2608, Key::Left),
    (2672, Key::Left),
    (2736, Key::Up),
    (2801, Key::Enter),
    (2865, Key::Enter),
    (2929, Key::Down),
    (2993, Key::Enter),
    (3058, Key::Right),
    (3122, Key::Enter),
    (3186, Key::Right),
    (3251, Key::Enter),
    (3315, Key::Right),
    (3379, Key::Enter),
    (3443, Key::Right),
    (3886, Key::Escape),
    (4100, Key::Enter),
    (4308, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run.
const MARKET_SHOTS: [(u64, &str); 60] = [
    (1689, "idle"),
    (1781, "k01-return"),
    (1847, "k02-down"),
    (1925, "k03-return"),
    (1956, "k04-return"),
    (2139, "k05-space"),
    (2174, "k06-return"),
    (2196, "fade-01"),
    (2206, "fade-02"),
    (2217, "fade-03"),
    (2228, "fade-04"),
    (2238, "fade-05"),
    (2249, "fade-06"),
    (2260, "fade-07"),
    (2271, "fade-08"),
    (2282, "fade-09"),
    (2292, "fade-10"),
    (2303, "fade-11"),
    (2314, "fade-12"),
    (2324, "fade-13"),
    (2335, "fade-14"),
    (2346, "fade-15"),
    (2392, "market"),
    (2422, "k07-left"),
    (2487, "k08-left"),
    (2551, "k09-left"),
    (2617, "k10-left"),
    (2680, "k11-left"),
    (2760, "k12-up"),
    (2832, "k13-return"),
    (2870, "k14-return"),
    (2971, "k15-down"),
    (3001, "k16-return"),
    (3065, "k17-right"),
    (3130, "k18-return"),
    (3194, "k19-right"),
    (3258, "k20-return"),
    (3323, "k21-right"),
    (3387, "k22-return"),
    (3485, "k23-right"),
    (3864, "messages-gone"),
    (3899, "k24-escape"),
    (3921, "back-01"),
    (3935, "back-02"),
    (3949, "back-03"),
    (3964, "back-04"),
    (3978, "back-05"),
    (3992, "back-06"),
    (4007, "back-07"),
    (4078, "shop-again"),
    (4285, "k25-return"),
    (4321, "k26-return"),
    (4342, "on-01"),
    (4357, "on-02"),
    (4371, "on-03"),
    (4385, "on-04"),
    (4399, "on-05"),
    (4414, "on-06"),
    (4428, "on-07"),
    (4550, "sign-up"),
];

/// The keys of `scripts/reference/offer.scenario` in the run of `docs/verification/m3c.md`
/// (seed 106): the test game through the Underground Market to the sign-up, a race chosen, the
/// hitman's offer and both its answers selected.
const OFFER_KEYS: [(u64, Key); 11] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2034, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2608, Key::Enter),
    (2950, Key::Right),
    (3015, Key::Left),
];

/// The ticks after which our frame equalled each screenshot of that run.
const OFFER_SHOTS: [(u64, &str); 24] = [
    (1691, "idle"),
    (1783, "k01-return"),
    (1847, "k02-down"),
    (1925, "k03-return"),
    (2011, "k04-return"),
    (2140, "k05-space"),
    (2393, "k06-return"),
    (2436, "k07-return"),
    (2458, "wipe-01"),
    (2479, "wipe-02"),
    (2586, "sign-up"),
    (2629, "k08-return"),
    (2651, "fill-01"),
    (2672, "fill-02"),
    (2693, "fill-03"),
    (2715, "fill-04"),
    (2736, "fill-05"),
    (2758, "fill-06"),
    (2801, "offer-01"),
    (2843, "offer-02"),
    (2886, "offer-03"),
    (2929, "question"),
    (2993, "k09-right"),
    (3058, "k10-left"),
];

/// The keys of `scripts/reference/preview.scenario` in the run of `docs/verification/m4a.md`:
/// the test game through the Underground Market to the sign-up, a race chosen, and the race's
/// preview wiping in.
const PREVIEW_KEYS: [(u64, Key); 10] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2608, Key::Enter),
    (2784, Key::Space),
];

/// The ticks after which our frame equalled each screenshot of that run.
const PREVIEW_SHOTS: [(u64, &str); 24] = [
    (1691, "idle"),
    (1783, "k01-return"),
    (1847, "k02-down"),
    (1926, "k03-return"),
    (2011, "k04-return"),
    (2140, "k05-space"),
    (2393, "k06-return"),
    (2436, "k07-return"),
    (2586, "sign-up"),
    (2629, "k08-return"),
    (2761, "filled"),
    (2790, "wipe-01"),
    (2797, "wipe-02"),
    (2804, "wipe-03"),
    (2811, "wipe-04"),
    (2818, "wipe-05"),
    (2825, "wipe-06"),
    (2828, "wipe-07"),
    (2828, "wipe-08"),
    (2828, "wipe-09"),
    (2828, "wipe-10"),
    (2828, "preview-01"),
    (2828, "preview-02"),
    (2828, "preview-03"),
];

/// The keys of `scripts/reference/race-start.scenario` in the run of `docs/verification/m4b.md`:
/// the preview run on to the race.
const RACE_START_KEYS: [(u64, Key); 10] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2608, Key::Enter),
    (2783, Key::Space),
];

/// The ticks after which our frame equalled each screenshot of that run: the preview held,
/// its fade, the race's intro and its countdown.
const RACE_START_SHOTS: [(u64, &str); 55] = [
    (2864, "r01"),
    (2871, "r02"),
    (2878, "r03"),
    (2885, "r04"),
    (2892, "r05"),
    (2899, "r06"),
    (2906, "r07"),
    (2913, "r08"),
    (2920, "r09"),
    (2927, "r10"),
    (2934, "r11"),
    (2941, "r12"),
    (2948, "r13"),
    (2955, "r14"),
    (2962, "r15"),
    (2969, "r16"),
    (2976, "r17"),
    (2983, "r18"),
    (2990, "r19"),
    (2998, "r20"),
    (3005, "r21"),
    (3011, "r22"),
    (3019, "r23"),
    (3026, "r24"),
    (3033, "r25"),
    (3041, "r26"),
    (3048, "r27"),
    (3054, "r28"),
    (3061, "r29"),
    (3068, "r30"),
    (3076, "r31"),
    (3083, "r32"),
    (3090, "r33"),
    (3098, "r34"),
    (3105, "r35"),
    (3112, "r36"),
    (3119, "r37"),
    (3126, "r38"),
    (3133, "r39"),
    (3140, "r40"),
    (3147, "r41"),
    (3155, "r42"),
    (3162, "r43"),
    (3169, "r44"),
    (3176, "r45"),
    (3183, "r46"),
    (3189, "r47"),
    (3196, "r48"),
    (3205, "r49"),
    (3212, "r50"),
    (3219, "r51"),
    (3226, "r52"),
    (3233, "r53"),
    (3240, "r54"),
    (3248, "r55"),
];

/// The keys held in `scripts/reference/drive.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`): Up from before the start for 324 ticks, Left and Right 14 ticks each on the way.
const DRIVE_HELD: [Held; 3] = [
    (3359, Key::Up, 324),
    (3540, Key::Left, 14),
    (3583, Key::Right, 14),
];

/// The ticks after which our frame equalled each screenshot of that run: the countdown, the
/// car pulling away with its wheels spinning, smoke and tire marks, the turns, the grass
/// slowing it, rolling out.
const DRIVE_SHOTS: [(u64, &str); 61] = [
    (3324, "race"),
    (3366, "d01"),
    (3374, "d02"),
    (3381, "d03"),
    (3389, "d04"),
    (3396, "d05"),
    (3403, "d06"),
    (3410, "d07"),
    (3417, "d08"),
    (3424, "d09"),
    (3432, "d10"),
    (3439, "d11"),
    (3446, "d12"),
    (3453, "d13"),
    (3460, "d14"),
    (3468, "d15"),
    (3475, "d16"),
    (3481, "d17"),
    (3489, "d18"),
    (3496, "d19"),
    (3503, "d20"),
    (3510, "d21"),
    (3517, "d22"),
    (3525, "d23"),
    (3532, "d24"),
    (3540, "d25"),
    (3546, "d26"),
    (3555, "d27"),
    (3560, "d28"),
    (3567, "d29"),
    (3574, "d30"),
    (3583, "d31"),
    (3589, "d32"),
    (3597, "d33"),
    (3603, "d34"),
    (3610, "d35"),
    (3617, "d36"),
    (3624, "d37"),
    (3632, "d38"),
    (3639, "d39"),
    (3646, "d40"),
    (3653, "d41"),
    (3660, "d42"),
    (3667, "d43"),
    (3673, "d44"),
    (3684, "d45"),
    (3689, "d46"),
    (3696, "d47"),
    (3703, "d48"),
    (3710, "d49"),
    (3716, "d50"),
    (3724, "d51"),
    (3730, "d52"),
    (3739, "d53"),
    (3746, "d54"),
    (3752, "d55"),
    (3759, "d56"),
    (3766, "d57"),
    (3774, "d58"),
    (3781, "d59"),
    (3789, "d60"),
];

/// The keys held in `scripts/reference/collide.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched): Down from before the start for 250 ticks, the car reversing
/// into the one behind, then Up for 57.
const COLLIDE_HELD: [Held; 2] = [(3359, Key::Down, 250), (3673, Key::Up, 57)];

/// The ticks after which our frame equalled each screenshot of that run: the car reversing,
/// knocking the car behind back again and again, pulling away.
const COLLIDE_SHOTS: [(u64, &str); 61] = [
    (3321, "race"),
    (3366, "c01"),
    (3373, "c02"),
    (3379, "c03"),
    (3386, "c04"),
    (3393, "c05"),
    (3400, "c06"),
    (3408, "c07"),
    (3415, "c08"),
    (3422, "c09"),
    (3429, "c10"),
    (3436, "c11"),
    (3442, "c12"),
    (3451, "c13"),
    (3457, "c14"),
    (3465, "c15"),
    (3472, "c16"),
    (3479, "c17"),
    (3486, "c18"),
    (3493, "c19"),
    (3500, "c20"),
    (3508, "c21"),
    (3515, "c22"),
    (3522, "c23"),
    (3529, "c24"),
    (3535, "c25"),
    (3543, "c26"),
    (3551, "c27"),
    (3558, "c28"),
    (3565, "c29"),
    (3572, "c30"),
    (3579, "c31"),
    (3586, "c32"),
    (3593, "c33"),
    (3600, "c34"),
    (3609, "c35"),
    (3613, "c36"),
    (3622, "c37"),
    (3627, "c38"),
    (3636, "c39"),
    (3643, "c40"),
    (3649, "c41"),
    (3657, "c42"),
    (3664, "c43"),
    (3671, "c44"),
    (3679, "c45"),
    (3686, "c46"),
    (3693, "c47"),
    (3700, "c48"),
    (3708, "c49"),
    (3715, "c50"),
    (3722, "c51"),
    (3730, "c52"),
    (3736, "c53"),
    (3743, "c54"),
    (3750, "c55"),
    (3758, "c56"),
    (3765, "c57"),
    (3772, "c58"),
    (3779, "c59"),
    (3786, "c60"),
];

/// The keys held in `scripts/reference/pickup.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched): the car standing until the first power-ups appear, then
/// Up for 120 ticks and Left for 6, over the turbo laid ahead of it.
const PICKUP_HELD: [Held; 2] = [(3651, Key::Up, 120), (3654, Key::Left, 6)];

/// The ticks after which our frame equalled each screenshot of that run: the power-ups
/// appearing, the car pulling away and taking the turbo, its pixels put back.
const PICKUP_SHOTS: [(u64, &str); 36] = [
    (3610, "u00"),
    (3617, "u01"),
    (3624, "u02"),
    (3631, "u03"),
    (3638, "u04"),
    (3645, "u05"),
    (3654, "u06"),
    (3660, "u07"),
    (3667, "u08"),
    (3674, "u09"),
    (3681, "u10"),
    (3688, "u11"),
    (3695, "u12"),
    (3702, "u13"),
    (3710, "u14"),
    (3717, "u15"),
    (3724, "u16"),
    (3731, "u17"),
    (3738, "u18"),
    (3745, "u19"),
    (3753, "u20"),
    (3760, "u21"),
    (3767, "u22"),
    (3774, "u23"),
    (3781, "u24"),
    (3788, "u25"),
    (3795, "u26"),
    (3802, "u27"),
    (3810, "u28"),
    (3817, "u29"),
    (3824, "u30"),
    (3831, "u31"),
    (3838, "u32"),
    (3845, "u33"),
    (3853, "u34"),
    (3860, "u35"),
];

/// The keys held in `scripts/reference/pedestrian.scenario`'s run of
/// `docs/verification/m4c.md` (`--no-ai`, its state watched): Up from before the start for
/// 468 ticks, Right for 50 through the first U-turn.
const PEDESTRIAN_HELD: [Held; 2] = [(3361, Key::Up, 468), (3511, Key::Right, 50)];

/// The ticks after which our frame equalled each screenshot of that run: the U-turn, two
/// pedestrians run over, the red tracks after them.
const PEDESTRIAN_SHOTS: [(u64, &str); 66] = [
    (3324, "race"),
    (3367, "h01"),
    (3374, "h02"),
    (3382, "h03"),
    (3389, "h04"),
    (3396, "h05"),
    (3403, "h06"),
    (3410, "h07"),
    (3417, "h08"),
    (3424, "h09"),
    (3432, "h10"),
    (3439, "h11"),
    (3446, "h12"),
    (3453, "h13"),
    (3460, "h14"),
    (3467, "h15"),
    (3474, "h16"),
    (3482, "h17"),
    (3489, "h18"),
    (3496, "h19"),
    (3503, "h20"),
    (3511, "h21"),
    (3517, "h22"),
    (3524, "h23"),
    (3532, "h24"),
    (3539, "h25"),
    (3546, "h26"),
    (3553, "h27"),
    (3562, "h28"),
    (3567, "h29"),
    (3574, "h30"),
    (3582, "h31"),
    (3589, "h32"),
    (3596, "h33"),
    (3603, "h34"),
    (3610, "h35"),
    (3617, "h36"),
    (3625, "h37"),
    (3632, "h38"),
    (3639, "h39"),
    (3647, "h40"),
    (3653, "h41"),
    (3660, "h42"),
    (3667, "h43"),
    (3675, "h44"),
    (3682, "h45"),
    (3689, "h46"),
    (3696, "h47"),
    (3703, "h48"),
    (3710, "h49"),
    (3717, "h50"),
    (3725, "h51"),
    (3732, "h52"),
    (3739, "h53"),
    (3746, "h54"),
    (3753, "h55"),
    (3760, "h56"),
    (3767, "h57"),
    (3774, "h58"),
    (3782, "h59"),
    (3789, "h60"),
    (3796, "h61"),
    (3803, "h62"),
    (3810, "h63"),
    (3817, "h64"),
    (3824, "h65"),
];

/// The keys held in `scripts/reference/guns.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched): Left from before the start for 96 ticks, turning the
/// standing car half round, then the gun key for 60.
const GUNS_HELD: [Held; 2] = [(3360, Key::Left, 96), (3625, Key::LeftCtrl, 60)];

/// The ticks after which our frame equalled each screenshot of that run: the car turned, its
/// gun firing at the car behind it, the flashes and the sparks, the target's damage.
const GUNS_SHOTS: [(u64, &str); 23] = [
    (3323, "race"),
    (3467, "turned"),
    (3610, "g00"),
    (3617, "g01"),
    (3623, "g02"),
    (3630, "g03"),
    (3638, "g04"),
    (3645, "g05"),
    (3652, "g06"),
    (3659, "g07"),
    (3666, "g08"),
    (3674, "g09"),
    (3681, "g10"),
    (3687, "g11"),
    (3695, "g12"),
    (3703, "g13"),
    (3710, "g14"),
    (3717, "g15"),
    (3723, "g16"),
    (3731, "g17"),
    (3738, "g18"),
    (3745, "g19"),
    (3753, "g20"),
];

/// The keys held in `scripts/reference/mines.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched, the test game with three mines): the mine key, again inside
/// its wait, Down backing over the mine, the mine key, then the accelerator with the horn.
const MINES_HELD: [Held; 6] = [
    (3640, Key::LeftAlt, 7),
    (3654, Key::LeftAlt, 7),
    (3697, Key::Down, 50),
    (3797, Key::LeftAlt, 7),
    (3812, Key::Up, 64),
    (3826, Key::Space, 43),
];

/// The ticks after which our frame equalled each screenshot of that run: the mine behind the
/// car, the blast's pictures, the crater, the mines left in the HUD, the horn's drive.
const MINES_SHOTS: [(u64, &str); 31] = [
    (3324, "race"),
    (3610, "m00"),
    (3647, "m01"),
    (3661, "m02"),
    (3682, "m03"),
    (3703, "m04"),
    (3710, "m05"),
    (3718, "m06"),
    (3725, "m07"),
    (3732, "m08"),
    (3739, "m09"),
    (3746, "m10"),
    (3753, "m11"),
    (3760, "m12"),
    (3767, "m13"),
    (3775, "m14"),
    (3782, "m15"),
    (3804, "h0"),
    (3811, "h1"),
    (3818, "h2"),
    (3826, "h3"),
    (3832, "h4"),
    (3839, "h5"),
    (3846, "h6"),
    (3853, "h7"),
    (3860, "h8"),
    (3869, "h9"),
    (3876, "h10"),
    (3882, "h11"),
    (3889, "h12"),
    (3896, "h13"),
];

/// The keys held in `scripts/reference/rocket.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched, the test game with a rocket): Up from the start, the turbo
/// with it for 71 ticks.
const ROCKET_HELD: [Held; 2] = [(3388, Key::Up, 122), (3403, Key::LeftShift, 71)];

/// The ticks after which our frame equalled each screenshot of that run: the rocket's flame
/// behind the car, its two pictures taking turns, gone with the turbo.
const ROCKET_SHOTS: [(u64, &str); 22] = [
    (3323, "race"),
    (3394, "r00"),
    (3403, "r01"),
    (3409, "r02"),
    (3416, "r03"),
    (3423, "r04"),
    (3430, "r05"),
    (3437, "r06"),
    (3444, "r07"),
    (3452, "r08"),
    (3459, "r09"),
    (3466, "r10"),
    (3474, "r11"),
    (3480, "r12"),
    (3487, "r13"),
    (3494, "r14"),
    (3502, "r15"),
    (3510, "r16"),
    (3516, "r17"),
    (3523, "r18"),
    (3530, "r19"),
    (3537, "r20"),
];

/// The keys held in `scripts/reference/wreck.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched, the test game at 99 % damage with a mine): the mine key,
/// Down backing over the mine, and Enter once the race is over.
const WRECK_HELD: [Held; 3] = [
    (3638, Key::LeftAlt, 7),
    (3695, Key::Down, 50),
    (4180, Key::Enter, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the blast wrecking
/// the car, its fire's pictures, the medals rolling to the new places, 300 ticks on the box
/// saying the race is over flying in and apart, and the view tilting away.
const WRECK_SHOTS: [(u64, &str); 85] = [
    (3323, "race"),
    (3610, "w00"),
    (3701, "w01"),
    (3708, "w02"),
    (3715, "w03"),
    (3723, "w04"),
    (3730, "w05"),
    (3737, "w06"),
    (3745, "w07"),
    (3751, "w08"),
    (3757, "w09"),
    (3765, "w10"),
    (3772, "w11"),
    (3780, "w12"),
    (3787, "w13"),
    (3793, "w14"),
    (3801, "w15"),
    (3808, "w16"),
    (3815, "w17"),
    (3822, "w18"),
    (3829, "w19"),
    (3837, "w20"),
    (3844, "w21"),
    (3850, "w22"),
    (3858, "w23"),
    (3865, "w24"),
    (3873, "w25"),
    (3880, "w26"),
    (3886, "w27"),
    (3894, "w28"),
    (3901, "w29"),
    (3907, "w30"),
    (3916, "w31"),
    (3922, "w32"),
    (3930, "w33"),
    (3937, "w34"),
    (3943, "w35"),
    (3951, "w36"),
    (3958, "w37"),
    (3967, "w38"),
    (3973, "w39"),
    (3979, "w40"),
    (3987, "w41"),
    (3994, "w42"),
    (4000, "w43"),
    (4008, "w44"),
    (4015, "w45"),
    (4023, "w46"),
    (4030, "w47"),
    (4037, "w48"),
    (4044, "w49"),
    (4051, "w50"),
    (4058, "w51"),
    (4065, "w52"),
    (4073, "w53"),
    (4080, "w54"),
    (4087, "w55"),
    (4094, "w56"),
    (4101, "w57"),
    (4108, "w58"),
    (4115, "w59"),
    (4123, "w60"),
    (4130, "w61"),
    (4137, "w62"),
    (4144, "w63"),
    (4151, "w64"),
    (4158, "w65"),
    (4165, "w66"),
    (4172, "w67"),
    (4179, "w68"),
    (4187, "w69"),
    (4193, "w70"),
    (4200, "w71"),
    (4207, "w72"),
    (4215, "w73"),
    (4222, "w74"),
    (4229, "w75"),
    (4236, "w76"),
    (4243, "w77"),
    (4250, "w78"),
    (4257, "w79"),
    (4265, "w80"),
    (4272, "w81"),
    (4279, "w82"),
    (4284, "w83"),
];

/// The keys held in `scripts/reference/spikes.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`, its state watched, the test game with spikes): Left from before the start
/// for 100 ticks, turning the standing car round, then Up into the car behind.
const SPIKES_HELD: [Held; 2] = [(3360, Key::Left, 100), (3481, Key::Up, 86)];

/// The ticks after which our frame equalled each screenshot of that run: the spiked car
/// turned, driving into the car behind and tearing into it.
const SPIKES_SHOTS: [(u64, &str); 27] = [
    (3324, "race"),
    (3467, "turned"),
    (3473, "s00"),
    (3480, "s01"),
    (3488, "s02"),
    (3495, "s03"),
    (3502, "s04"),
    (3509, "s05"),
    (3516, "s06"),
    (3523, "s07"),
    (3530, "s08"),
    (3538, "s09"),
    (3545, "s10"),
    (3552, "s11"),
    (3559, "s12"),
    (3568, "s13"),
    (3573, "s14"),
    (3580, "s15"),
    (3588, "s16"),
    (3595, "s17"),
    (3602, "s18"),
    (3609, "s19"),
    (3616, "s20"),
    (3623, "s21"),
    (3630, "s22"),
    (3638, "s23"),
    (3646, "s24"),
];

/// The keys held in `scripts/reference/help.scenario`'s run of `docs/verification/m4c.md`
/// (`--no-ai`): F1 after the start, Enter on each of the help's pages, Escape, F1 in the pause
/// box, Enter on each page again.
const HELP_HELD: [Held; 7] = [
    (3465, Key::F1, 7),
    (3675, Key::Enter, 7),
    (3885, Key::Enter, 7),
    (4100, Key::Escape, 7),
    (4340, Key::F1, 7),
    (4585, Key::Enter, 7),
    (4795, Key::Enter, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the race fading out,
/// the keys' page with the controls' keys, the tips' page, the race fading back in; the pause
/// box flying in and apart on F1, and the help again.
const HELP_SHOTS: [(u64, &str); 224] = [
    (3324, "race"),
    (3453, "h000"),
    (3460, "h001"),
    (3467, "h002"),
    (3474, "h003"),
    (3480, "h004"),
    (3487, "h005"),
    (3494, "h006"),
    (3501, "h007"),
    (3508, "h008"),
    (3515, "h009"),
    (3523, "h010"),
    (3530, "h011"),
    (3537, "h012"),
    (3544, "h013"),
    (3551, "h014"),
    (3558, "h015"),
    (3565, "h016"),
    (3572, "h017"),
    (3580, "h018"),
    (3587, "h019"),
    (3593, "h020"),
    (3600, "h021"),
    (3607, "h022"),
    (3614, "h023"),
    (3621, "h024"),
    (3628, "h025"),
    (3635, "h026"),
    (3642, "h027"),
    (3649, "h028"),
    (3656, "h029"),
    (3663, "h030"),
    (3670, "h031"),
    (3676, "h032"),
    (3683, "h033"),
    (3689, "h034"),
    (3696, "h035"),
    (3703, "h036"),
    (3710, "h037"),
    (3718, "h038"),
    (3725, "h039"),
    (3732, "h040"),
    (3739, "h041"),
    (3746, "h042"),
    (3753, "h043"),
    (3760, "h044"),
    (3767, "h045"),
    (3775, "h046"),
    (3782, "h047"),
    (3789, "h048"),
    (3796, "h049"),
    (3803, "h050"),
    (3810, "h051"),
    (3817, "h052"),
    (3824, "h053"),
    (3831, "h054"),
    (3838, "h055"),
    (3845, "h056"),
    (3852, "h057"),
    (3859, "h058"),
    (3866, "h059"),
    (3873, "h060"),
    (3880, "h061"),
    (3886, "h062"),
    (3893, "h063"),
    (3899, "h064"),
    (3906, "h065"),
    (3913, "h066"),
    (3921, "h067"),
    (3928, "h068"),
    (3935, "h069"),
    (3942, "h070"),
    (3949, "h071"),
    (3956, "h072"),
    (3963, "h073"),
    (3971, "h074"),
    (3978, "h075"),
    (3985, "h076"),
    (3992, "h077"),
    (3999, "h078"),
    (4006, "h079"),
    (4013, "h080"),
    (4020, "h081"),
    (4028, "h082"),
    (4034, "h083"),
    (4041, "h084"),
    (4048, "h085"),
    (4055, "h086"),
    (4063, "h087"),
    (4069, "h088"),
    (4078, "h089"),
    (4083, "h090"),
    (4090, "h091"),
    (4097, "h092"),
    (4108, "h093"),
    (4113, "h094"),
    (4121, "h095"),
    (4128, "h096"),
    (4135, "h097"),
    (4142, "h098"),
    (4149, "h099"),
    (4156, "h100"),
    (4163, "h101"),
    (4171, "h102"),
    (4178, "h103"),
    (4185, "h104"),
    (4192, "h105"),
    (4199, "h106"),
    (4206, "h107"),
    (4214, "h108"),
    (4221, "h109"),
    (4223, "h110"),
    (4230, "h111"),
    (4237, "h112"),
    (4244, "h113"),
    (4251, "h114"),
    (4258, "h115"),
    (4265, "h116"),
    (4272, "h117"),
    (4279, "h118"),
    (4286, "h119"),
    (4293, "h120"),
    (4300, "h121"),
    (4307, "h122"),
    (4314, "h123"),
    (4321, "h124"),
    (4328, "h125"),
    (4335, "h126"),
    (4342, "h127"),
    (4348, "h128"),
    (4354, "h129"),
    (4361, "h130"),
    (4368, "h131"),
    (4375, "h132"),
    (4382, "h133"),
    (4389, "h134"),
    (4397, "h135"),
    (4404, "h136"),
    (4411, "h137"),
    (4418, "h138"),
    (4425, "h139"),
    (4432, "h140"),
    (4439, "h141"),
    (4446, "h142"),
    (4453, "h143"),
    (4461, "h144"),
    (4468, "h145"),
    (4475, "h146"),
    (4482, "h147"),
    (4489, "h148"),
    (4496, "h149"),
    (4503, "h150"),
    (4511, "h151"),
    (4518, "h152"),
    (4525, "h153"),
    (4531, "h154"),
    (4538, "h155"),
    (4545, "h156"),
    (4552, "h157"),
    (4559, "h158"),
    (4566, "h159"),
    (4573, "h160"),
    (4580, "h161"),
    (4586, "h162"),
    (4594, "h163"),
    (4600, "h164"),
    (4607, "h165"),
    (4614, "h166"),
    (4621, "h167"),
    (4628, "h168"),
    (4635, "h169"),
    (4642, "h170"),
    (4649, "h171"),
    (4657, "h172"),
    (4664, "h173"),
    (4671, "h174"),
    (4678, "h175"),
    (4685, "h176"),
    (4692, "h177"),
    (4700, "h178"),
    (4707, "h179"),
    (4713, "h180"),
    (4720, "h181"),
    (4727, "h182"),
    (4734, "h183"),
    (4741, "h184"),
    (4748, "h185"),
    (4755, "h186"),
    (4762, "h187"),
    (4769, "h188"),
    (4776, "h189"),
    (4783, "h190"),
    (4790, "h191"),
    (4796, "h192"),
    (4803, "h193"),
    (4809, "h194"),
    (4816, "h195"),
    (4823, "h196"),
    (4830, "h197"),
    (4837, "h198"),
    (4845, "h199"),
    (4852, "h200"),
    (4859, "h201"),
    (4866, "h202"),
    (4873, "h203"),
    (4880, "h204"),
    (4887, "h205"),
    (4895, "h206"),
    (4902, "h207"),
    (4909, "h208"),
    (4916, "h209"),
    (4923, "h210"),
    (4930, "h211"),
    (4937, "h212"),
    (4944, "h213"),
    (4951, "h214"),
    (4958, "h215"),
    (4965, "h216"),
    (4973, "h217"),
    (4979, "h218"),
    (4988, "h219"),
    (4993, "h220"),
    (5000, "h221"),
    (5007, "h222"),
];

/// The keys held in `scripts/reference/pause.scenario`'s run of `docs/verification/m4b.md`:
/// Escape pauses the race, N ends the pause; held 100 ms (7 ticks), as the race and the pause
/// read the keys held.
const PAUSE_HELD: [Held; 2] = [(3395, Key::Escape, 7), (3610, Key::N, 7)];

/// The ticks after which our frame equalled each screenshot of that run: the box flying in,
/// landed, flying apart, and the race going on.
const PAUSE_SHOTS: [(u64, &str); 50] = [
    (3403, "e01"),
    (3409, "e02"),
    (3416, "e03"),
    (3423, "e04"),
    (3430, "e05"),
    (3437, "e06"),
    (3444, "e07"),
    (3451, "e08"),
    (3458, "e09"),
    (3466, "e10"),
    (3473, "e11"),
    (3480, "e12"),
    (3487, "e13"),
    (3494, "e14"),
    (3501, "e15"),
    (3509, "e16"),
    (3516, "e17"),
    (3523, "e18"),
    (3530, "e19"),
    (3537, "e20"),
    (3544, "e21"),
    (3551, "e22"),
    (3558, "e23"),
    (3565, "e24"),
    (3572, "e25"),
    (3617, "n01"),
    (3623, "n02"),
    (3630, "n03"),
    (3637, "n04"),
    (3644, "n05"),
    (3651, "n06"),
    (3658, "n07"),
    (3666, "n08"),
    (3673, "n09"),
    (3680, "n10"),
    (3687, "n11"),
    (3694, "n12"),
    (3702, "n13"),
    (3709, "n14"),
    (3716, "n15"),
    (3723, "n16"),
    (3730, "n17"),
    (3737, "n18"),
    (3744, "n19"),
    (3751, "n20"),
    (3758, "n21"),
    (3765, "n22"),
    (3772, "n23"),
    (3779, "n24"),
    (3786, "n25"),
];

/// The keys held in `scripts/reference/abort.scenario`'s run: Escape, then Y.
const ABORT_HELD: [Held; 2] = [(3397, Key::Escape, 7), (3576, Key::Y, 7)];

/// The ticks after which our frame equalled each screenshot of that run: the box landed,
/// flying apart, the race's last frame and the view tilting away (spec M4c).
const ABORT_SHOTS: [(u64, &str); 16] = [
    (3555, "box"),
    (3584, "a01"),
    (3589, "a02"),
    (3597, "a03"),
    (3604, "a04"),
    (3611, "a05"),
    (3618, "a06"),
    (3625, "a07"),
    (3632, "a08"),
    (3639, "a09"),
    (3648, "a10"),
    (3655, "a11"),
    (3662, "a12"),
    (3669, "a13"),
    (3676, "a14"),
    (3680, "a15"),
];

/// The keys of `scripts/reference/reversed.scenario` in the run of `docs/verification/m4b.md`:
/// the preview run with the medium race chosen (Right at the sign-up), circuit 17.
const REVERSED_KEYS: [(u64, Key); 11] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2597, Key::Right),
    (2608, Key::Enter),
    (2783, Key::Space),
];

/// The ticks after which our frame equalled each screenshot of that run: the preview held,
/// its fade, the reversed track's intro and countdown.
const REVERSED_SHOTS: [(u64, &str); 67] = [
    (2862, "r01"),
    (2869, "r02"),
    (2876, "r03"),
    (2883, "r04"),
    (2890, "r05"),
    (2897, "r06"),
    (2904, "r07"),
    (2911, "r08"),
    (2918, "r09"),
    (2925, "r10"),
    (2932, "r11"),
    (2939, "r12"),
    (2946, "r13"),
    (2953, "r14"),
    (2960, "r15"),
    (2967, "r16"),
    (2974, "r17"),
    (2981, "r18"),
    (2988, "r19"),
    (2995, "r20"),
    (3003, "r21"),
    (3010, "r22"),
    (3016, "r23"),
    (3023, "r24"),
    (3030, "r25"),
    (3037, "r26"),
    (3044, "r27"),
    (3052, "r28"),
    (3059, "r29"),
    (3066, "r30"),
    (3073, "r31"),
    (3080, "r32"),
    (3087, "r33"),
    (3094, "r34"),
    (3102, "r35"),
    (3109, "r36"),
    (3116, "r37"),
    (3123, "r38"),
    (3130, "r39"),
    (3137, "r40"),
    (3144, "r41"),
    (3151, "r42"),
    (3159, "r43"),
    (3166, "r44"),
    (3173, "r45"),
    (3180, "r46"),
    (3187, "r47"),
    (3194, "r48"),
    (3202, "r49"),
    (3209, "r50"),
    (3216, "r51"),
    (3223, "r52"),
    (3230, "r53"),
    (3237, "r54"),
    (3244, "r55"),
    (3252, "r56"),
    (3259, "r57"),
    (3266, "r58"),
    (3273, "r59"),
    (3280, "r60"),
    (3287, "r61"),
    (3294, "r62"),
    (3301, "r63"),
    (3309, "r64"),
    (3316, "r65"),
    (3323, "r66"),
    (3330, "r67"),
];

/// The keys of `scripts/reference/sabotage.scenario` in the run of `docs/verification/m3c.md`
/// (its sabotage's clock fixed at 31375): the test game, the sabotage bought in the Underground
/// Market, the sign-up and the sabotage's popup.
const SABOTAGE_KEYS: [(u64, Key); 12] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1951, Key::Enter),
    (2035, Key::Space),
    (2161, Key::Enter),
    (2416, Key::Left),
    (2479, Key::Enter),
    (2547, Key::Right),
    (2611, Key::Enter),
    (2803, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run.
const SABOTAGE_SHOTS: [(u64, &str); 23] = [
    (1693, "idle"),
    (1785, "k01-return"),
    (1849, "k02-down"),
    (1929, "k03-return"),
    (2011, "k04-return"),
    (2141, "k05-space"),
    (2394, "k06-return"),
    (2459, "k07-left"),
    (2523, "k08-return"),
    (2587, "k09-right"),
    (2630, "k10-return"),
    (2652, "wipe-01"),
    (2673, "wipe-02"),
    (2780, "sign-up"),
    (2823, "k11-return"),
    (2845, "fill-01"),
    (2866, "fill-02"),
    (2887, "fill-03"),
    (2909, "fill-04"),
    (2930, "fill-05"),
    (2952, "fill-06"),
    (2995, "popup-01"),
    (3066, "popup-02"),
];

/// The keys of `scripts/reference/quick-save.scenario` in the run of
/// `docs/verification/m3c.md`: the test game, F2 held in the shop, an engine bought, F3 held,
/// the Underground Market and F2 there.
const QUICK_KEYS: [(u64, Key); 19] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2034, Key::Space),
    (2226, Key::Space),
    (2291, Key::Left),
    (2355, Key::Left),
    (2419, Key::Left),
    (2483, Key::Left),
    (2547, Key::Enter),
    (2676, Key::Space),
    (2741, Key::Right),
    (2805, Key::Right),
    (2869, Key::Right),
    (2933, Key::Right),
    (2997, Key::Enter),
    (3319, Key::Space),
];

const QUICK_HELD: [(u64, Key, u64); 3] = [
    (2162, Key::F2, 22),
    (2611, Key::F3, 22),
    (3254, Key::F2, 22),
];

/// The ticks after which our frame equalled each screenshot of that run.
const QUICK_SHOTS: [(u64, &str); 22] = [
    (1690, "idle"),
    (1783, "k01-return"),
    (1847, "k02-down"),
    (1926, "k03-return"),
    (2011, "k04-return"),
    (2140, "k05-space"),
    (2204, "k06-f2"),
    (2268, "k07-space"),
    (2332, "k08-left"),
    (2397, "k09-left"),
    (2461, "k10-left"),
    (2525, "k11-left"),
    (2590, "k12-return"),
    (2654, "k13-f3"),
    (2718, "k14-space"),
    (2782, "k15-right"),
    (2847, "k16-right"),
    (2911, "k17-right"),
    (2975, "k18-right"),
    (3232, "k19-return"),
    (3297, "k20-f2"),
    (3361, "k21-space"),
];

/// The seed the reference runs were made with (`scripts/reference-run.sh --seed 1`).
const SEED: u32 = 1;

/// One line per screenshot (the frame's pixels and palette), one for the run's sound and one
/// for the last `dr.cfg` it wrote. The run stops at `ticks`, or earlier when the game quits.
fn manifest(keys: &[(u64, Key)], shots: &[(u64, &str)], ticks: u64) -> String {
    manifest_with(keys, shots, ticks, Vec::new())
}

/// [`manifest`] with saved games in the slots, and a line for each game the run saves.
fn manifest_with(
    keys: &[(u64, Key)],
    shots: &[(u64, &str)],
    ticks: u64,
    slots: Vec<Option<Vec<u8>>>,
) -> String {
    manifest_seeded((SEED, None), (keys, &[]), shots, ticks, slots)
}

/// A key held down: the tick it goes down, the key, the ticks it stays down.
type Held = (u64, Key, u64);

/// [`manifest_with`] for a run seeded with `seed` (its sabotage's clock fixed at `clock`),
/// with `held` keys besides the ones pressed and let go at once.
fn manifest_seeded(
    (seed, clock): (u32, Option<u32>),
    (keys, held): (&[(u64, Key)], &[Held]),
    shots: &[(u64, &str)],
    ticks: u64,
    slots: Vec<Option<Vec<u8>>>,
) -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let config = assets.menu.default_config.clone();
    let mut game = Game::with_seed(assets, config, seed);
    game.set_saved_games(slots);
    if let Some(ms) = clock {
        game.fix_sabotage_clock(ms);
    }
    let mut saved = Vec::new();
    let mut lines = String::new();
    let mut audio = Vec::new();
    let mut written = None;
    let mut done = 0;
    while !game.quit_requested() && done < ticks {
        for &(_, key) in keys.iter().filter(|(at, _)| *at == done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        for &(at, key, ticks) in held {
            if at == done || at + ticks == done {
                game.input(InputEvent::Key {
                    key,
                    pressed: at == done,
                });
            }
        }
        game.tick();
        game.take_audio(&mut audio);
        written = game.take_config().or(written);
        saved.extend(game.take_saved_game());
        done += 1;
        for &(_, name) in shots.iter().filter(|(at, _)| *at == done) {
            let frame = game.frame();
            let mut hasher = Sha256::new();
            hasher.update(frame.pixels);
            hasher.update(frame.palette.as_flattened());
            writeln!(lines, "{}  frame after tick {done} ({name})", hex(hasher)).unwrap();
        }
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  sound of {done} ticks", hex(hasher)).unwrap();
    for (slot, file) in saved {
        if let Some(dir) = std::env::var_os("DEADRALLY_DUMP_SAVES") {
            std::fs::write(
                std::path::Path::new(&dir).join(format!("DR.SG{slot}")),
                &file,
            )
            .unwrap();
        }
        let mut hasher = Sha256::new();
        hasher.update(&file);
        writeln!(lines, "{}  saved game in slot {slot}", hex(hasher)).unwrap();
    }
    let written = written.expect("dr.cfg is written at start-up");
    if let Some(path) = std::env::var_os("DEADRALLY_DUMP_CFG") {
        std::fs::write(path, &written).unwrap();
    }
    let mut hasher = Sha256::new();
    hasher.update(written);
    writeln!(lines, "{}  dr.cfg written last", hex(hasher)).unwrap();
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_menu_run_matches_the_committed_manifest() {
    // The manifest was written after every screenshot of the run equalled our frame at its
    // tick and the sound was measured against the recording (docs/verification/m2a.md); a
    // change to the menu must not alter a frame or a sound unnoticed.
    let lines = manifest(&KEYS, &SHOTS, MAX_TICKS);
    assert!(
        lines.contains("after tick 8937"),
        "the end screen asks to quit after its fade"
    );
    check_manifest("menu-run.sha256", &lines, "the menu run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_configure_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and the
    // dr.cfg written last equalled the original's but for its random byte
    // (docs/verification/m2b.md).
    let lines = manifest(&CONFIGURE_KEYS, &CONFIGURE_SHOTS, 7_900);
    check_manifest("configure-run.sha256", &lines, "the configure run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_hall_of_fame_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick and the
    // recording of the run compared as M1b's (docs/verification/m2c.md).
    let lines = manifest(&HALL_OF_FAME_KEYS, &HALL_OF_FAME_SHOTS, 7_600);
    check_manifest("hall-of-fame-run.sha256", &lines, "the hall of fame run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_new_game_run_matches_the_committed_manifest() {
    // Written after every screenshot of the seed-1 run equalled our frame at its tick
    // (docs/verification/m3a.md): the licence, the drivers and the races they sign up for
    // follow the original's random numbers; a change to any of them shows here.
    let lines = manifest(&NEW_GAME_KEYS, &NEW_GAME_SHOTS, 3_300);
    check_manifest("new-game-run.sha256", &lines, "the new game run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_saved_games_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick and the game
    // it saved equalled the original's file byte for byte (docs/verification/m3b.md).
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_with(&SAVED_GAMES_KEYS, &SAVED_GAMES_SHOTS, 3_700, slots);
    assert!(
        lines.contains("saved game in slot 1"),
        "the run saves into slot 1"
    );
    check_manifest("saved-games-run.sha256", &lines, "the saved games run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_market_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m3c.md): the market's fades, the loan shark's deals, the weapons'
    // prices and stock, and the money after each.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_with(&MARKET_KEYS, &MARKET_SHOTS, 4_600, slots);
    check_manifest("market-run.sha256", &lines, "the market run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_sabotage_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m3c.md): the sabotage's victim, its damage from the clock, the popup.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, Some(31_375)),
        (&SABOTAGE_KEYS, &[]),
        &SABOTAGE_SHOTS,
        3_100,
        slots,
    );
    check_manifest("sabotage-run.sha256", &lines, "the sabotage run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_preview_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4a.md): the grid's drivers, the circuit, its laps and prize, the wipe.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&PREVIEW_KEYS, &[]),
        &PREVIEW_SHOTS,
        2_900,
        slots,
    );
    check_manifest("preview-run.sha256", &lines, "the preview run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_race_start_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4b.md): the preview held and faded, the race's tilted zoom, the
    // player's car's flash, the colours' return, the lights' countdown, the pedestrians.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &[]),
        &RACE_START_SHOTS,
        3_260,
        slots,
    );
    check_manifest("race-start-run.sha256", &lines, "the race-start run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_pause_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4b.md): the box's tiles from their random starts, the box landed,
    // N, the tiles thrown apart and falling, the race going on.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &PAUSE_HELD),
        &PAUSE_SHOTS,
        3_800,
        slots,
    );
    check_manifest("pause-run.sha256", &lines, "the pause run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_drive_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4c.md): a wrong step of the car's physics, its wheelspin, marks or
    // the power-ups' draws of `rand()` moves or turns the car on screen within a second.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &DRIVE_HELD),
        &DRIVE_SHOTS,
        3_800,
        slots,
    );
    check_manifest("drive-run.sha256", &lines, "the drive run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_collide_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // wrong push, knock or spin between cars moves them apart differently within ticks.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &COLLIDE_HELD),
        &COLLIDE_SHOTS,
        3_800,
        slots,
    );
    check_manifest("collide-run.sha256", &lines, "the collide run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_pickup_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // wrong draw of `rand()` lays the power-ups elsewhere, a wrong pick-up leaves the turbo.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &PICKUP_HELD),
        &PICKUP_SHOTS,
        3_860,
        slots,
    );
    check_manifest("pickup-run.sha256", &lines, "the pickup run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_pedestrian_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // wrong jolt or draw of `rand()` sends the car elsewhere after the first pedestrian.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &PEDESTRIAN_HELD),
        &PEDESTRIAN_SHOTS,
        3_900,
        slots,
    );
    check_manifest("pedestrian-run.sha256", &lines, "the pedestrian run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_mines_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // mine in the wrong place, a wrong blast or a horn that brakes shows here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(armed_save(&assets.menu.texts, 37, [3, 0, 0]));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &MINES_HELD),
        &MINES_SHOTS,
        3_900,
        slots,
    );
    check_manifest("mines-run.sha256", &lines, "the mines run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_rocket_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // flame in the wrong place or turning at the wrong time shows here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(armed_save(&assets.menu.texts, 37, [0, 0, 1]));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &ROCKET_HELD),
        &ROCKET_SHOTS,
        3_540,
        slots,
    );
    check_manifest("rocket-run.sha256", &lines, "the rocket run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_wreck_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // wreck's fire, the HUD's medals rolling to the new places, the race's end 300 ticks on
    // and the view tilting away show here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(armed_save(&assets.menu.texts, 99, [1, 0, 0]));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &WRECK_HELD),
        &WRECK_SHOTS,
        4_290,
        slots,
    );
    check_manifest("wreck-run.sha256", &lines, "the wreck run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_spikes_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): the
    // spiked wheels' sprites or what they tear off the other car show here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(armed_save(&assets.menu.texts, 37, [0, 1, 0]));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &SPIKES_HELD),
        &SPIKES_SHOTS,
        3_650,
        slots,
    );
    check_manifest("spikes-run.sha256", &lines, "the spikes run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_help_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4c.md): the fades' 63 ticks, the keys' page naming the controls'
    // keys and the gamepad's inputs, the tips' page, and the race back in its own palette.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &HELP_HELD),
        &HELP_SHOTS,
        5_020,
        slots,
    );
    check_manifest("help-run.sha256", &lines, "the help run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_guns_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m4c.md): a
    // wrong muzzle, spread or damage shows within a frame of the first shot.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &GUNS_HELD),
        &GUNS_SHOTS,
        3_760,
        slots,
    );
    check_manifest("guns-run.sha256", &lines, "the guns run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_abort_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4b.md, m4c.md): the box landed, Y, the tiles flying apart, the race's
    // last frame and the view tilting away. The run goes on into the stand-in's shop, so its
    // sound pins the menus' music coming back after the race.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &ABORT_HELD),
        &ABORT_SHOTS,
        3_800,
        slots,
    );
    check_manifest("abort-run.sha256", &lines, "the abort run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_reversed_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4b.md): the ninth track turned half round, its own palette, the
    // cars facing back, the scene lit as before turning, the countdown.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&REVERSED_KEYS, &[]),
        &REVERSED_SHOTS,
        3_340,
        slots,
    );
    check_manifest("reversed-run.sha256", &lines, "the reversed run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_offer_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m3c.md): the hitman's chance, his victim and pay, his picture, the
    // waits before the question and the question's answers.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded((106, None), (&OFFER_KEYS, &[]), &OFFER_SHOTS, 3_080, slots);
    check_manifest("offer-run.sha256", &lines, "the offer run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_quick_save_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick and the
    // quicksave our run writes equalled the original's DR.SG7 byte for byte
    // (docs/verification/m3c.md).
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (1, None),
        (&QUICK_KEYS, &QUICK_HELD),
        &QUICK_SHOTS,
        3_380,
        slots,
    );
    check_manifest("quick-save-run.sha256", &lines, "the quick save run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_shop_purchases_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m3c.md): prices, refunds, messages and the money after each.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_with(&PURCHASES_KEYS, &PURCHASES_SHOTS, 5_300, slots);
    check_manifest(
        "shop-purchases-run.sha256",
        &lines,
        "the shop purchases run",
    );
}

/// The saved game `scripts/reference/saved-games.scenario` starts from (the reference run
/// loads the same file): seed 1's drivers as `initDrivers` sets them up, recomputed here, and
/// a player part-way through a game.
fn test_save(texts: &deadrally_gamedata::text::Texts) -> Vec<u8> {
    armed_save(texts, 37, [0, 0, 0])
}

/// The test game with the player's damage (37 in it), mines, spikes and rocket
/// (`captures/test.sg` with them changed).
fn armed_save(
    texts: &deadrally_gamedata::text::Texts,
    damage: i32,
    [mines, spikes, rocket]: [i32; 3],
) -> Vec<u8> {
    let campaign = &texts.campaign;
    let mut state: u32 = 1;
    let mut rand = || {
        state = state.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((state >> 16) & 0x7FFF) as i32
    };
    const CARS: [i32; 19] = [5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 0, 0];
    const POINTS: [i32; 19] = [
        100, 86, 77, 69, 63, 57, 51, 46, 42, 37, 33, 28, 25, 20, 17, 13, 9, 5, 2,
    ];
    const PLAYER_FACE: usize = 5;
    // name, then damage, engine, tires, armour, car, 3 unused, colour, money, loan, loan
    // races, car's price, face, points, rank, wins, races, last and total income, mines,
    // spikes, rocket, sabotage.
    let record = |name: &[u8], numbers: [i32; 24]| {
        let mut bytes = name.to_vec();
        bytes.resize(12, 0);
        for number in numbers {
            bytes.extend_from_slice(&number.to_le_bytes());
        }
        bytes
    };
    let mut drivers = Vec::new();
    let mut taken = [false; 20];
    for index in 0..19 {
        let car = CARS[index];
        let spec = campaign.cars[car as usize];
        let money = rand() % 100_000;
        let engine = rand() % spec.upgrades[0];
        let tires = rand() % spec.upgrades[1];
        let armour = rand() % spec.upgrades[2];
        let mut face = index;
        while taken[face] || face == PLAYER_FACE {
            face += 1;
        }
        taken[face] = true;
        let f = face as i32;
        drivers.extend(record(
            &campaign.driver_names[face],
            [
                0,
                engine,
                tires,
                armour,
                car,
                0,
                0,
                0,
                f,
                money,
                0,
                0,
                spec.price,
                f,
                POINTS[index],
                index as i32 + 1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
        ));
    }
    let price = campaign.cars[1].price;
    drivers.extend(record(
        b"Tester",
        [
            damage, 1, 1, 0, 1, 0, 0, 0, 60, 23456, -1, -1, price, 5, 41, 12, 0, 0, 0, 0, mines,
            spikes, rocket, 0,
        ],
    ));
    let mut name = [0; 15];
    name[..9].copy_from_slice(b"save test");
    deadrally_gamedata::save_game::SaveGame {
        driver_id: 19,
        use_weapons: 1,
        difficulty: 1,
        name,
        drivers,
    }
    .encode(77)
}
