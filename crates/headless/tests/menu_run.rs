//! A run through the main menu against the developer's real game data. Run with
//! `cargo test-data`; the test reads DEADRALLY_DATA and fails (never passes silently) when it is
//! unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg::DrCfg;
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

/// The keys of `scripts/reference/results.scenario`'s run of `docs/verification/m5.md`: the
/// race start's (the space ending the sign-up's linger four waits later in this run, as the
/// menu's pulse after the race shows), the mine, Down backing over it and Enter on the race's
/// end as in the wreck run, then Enter after the medium race's page, after the hard race's and
/// after the statistics; in the shop after them, Enter on the last place's popup and on the way
/// on, then twice on the Underground Market's way on with the car wrecked. From the statistics
/// on, the keys are where the original's count of waits (0x456BA0, read from its memory) puts
/// them, not where its pulse alone would: the pulse comes round every 34 waits, the background's
/// copper only every 70 (docs/verification/m5.md).
const RESULTS_KEYS: [(u64, Key); 17] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2608, Key::Enter),
    (2787, Key::Space),
    (4470, Key::Enter),
    (4827, Key::Enter),
    (5182, Key::Enter),
    (5539, Key::Enter),
    (5896, Key::Enter),
    (6253, Key::Enter),
    (6611, Key::Enter),
];
const RESULTS_HELD: [Held; 3] = [
    (3643, Key::LeftAlt, 7),
    (3700, Key::Down, 50),
    (4184, Key::Enter, 7),
];

/// The ticks after which our frame equalled each screenshot of that run, one every 200 ms
/// from the race's end on: the box, the view tilting away, the results fading in, the three
/// races' pages, the statistics and the way out (s035 and s064, caught as the screen changed,
/// left out; and s085 to s092, as the original stood still after the statistics' Enter;
/// docs/verification/m5.md); then the shop fading in, the last place's popup with its cursor,
/// the shop drawn again, the Underground Market and its refusals (but for s110, s166, s171
/// and s176, a wait or two off).
const RESULTS_SHOTS: [(u64, &str); 207] = [
    (3901, "s000"),
    (3901, "s001"),
    (3901, "s002"),
    (3901, "s003"),
    (3901, "s004"),
    (3901, "s005"),
    (3901, "s006"),
    (3901, "s007"),
    (3901, "s008"),
    (3901, "s009"),
    (3901, "s010"),
    (3901, "s011"),
    (3901, "s012"),
    (3901, "s013"),
    (3901, "s014"),
    (3901, "s015"),
    (4197, "s016"),
    (4211, "s017"),
    (4225, "s018"),
    (4240, "s019"),
    (4254, "s020"),
    (4268, "s021"),
    (4282, "s022"),
    (4292, "s023"),
    (4306, "s024"),
    (4320, "s025"),
    (4335, "s026"),
    (4349, "s027"),
    (4359, "s028"),
    (4377, "s029"),
    (4392, "s030"),
    (4350, "s031"),
    (4352, "s032"),
    (4389, "s033"),
    (4375, "s034"),
    (4478, "s036"),
    (4492, "s037"),
    (4506, "s038"),
    (4474, "s039"),
    (4527, "s040"),
    (4549, "s041"),
    (4563, "s042"),
    (4553, "s043"),
    (4592, "s044"),
    (4592, "s045"),
    (4552, "s046"),
    (4563, "s047"),
    (4547, "s048"),
    (4595, "s049"),
    (4554, "s050"),
    (4692, "s051"),
    (4706, "s052"),
    (4716, "s053"),
    (4735, "s054"),
    (4687, "s055"),
    (4763, "s056"),
    (4761, "s057"),
    (4758, "s058"),
    (4800, "s059"),
    (4785, "s060"),
    (4829, "s061"),
    (4842, "s062"),
    (4862, "s063"),
    (4833, "s065"),
    (4909, "s066"),
    (4923, "s067"),
    (4926, "s068"),
    (4946, "s069"),
    (4898, "s070"),
    (4913, "s071"),
    (4927, "s072"),
    (4907, "s073"),
    (4909, "s074"),
    (4928, "s075"),
    (5050, "s076"),
    (5066, "s077"),
    (5081, "s078"),
    (5061, "s079"),
    (5041, "s080"),
    (5047, "s081"),
    (5066, "s082"),
    (5084, "s083"),
    (5038, "s084"),
    (5295, "s093"),
    (5309, "s094"),
    (5323, "s095"),
    (5338, "s096"),
    (5352, "s097"),
    (5366, "s098"),
    (5381, "s099"),
    (5395, "s100"),
    (5409, "s101"),
    (5423, "s102"),
    (5438, "s103"),
    (5452, "s104"),
    (5466, "s105"),
    (5481, "s106"),
    (5495, "s107"),
    (5509, "s108"),
    (5523, "s109"),
    (5552, "s111"),
    (5566, "s112"),
    (5581, "s113"),
    (5595, "s114"),
    (5609, "s115"),
    (5624, "s116"),
    (5638, "s117"),
    (5652, "s118"),
    (5666, "s119"),
    (5681, "s120"),
    (5695, "s121"),
    (5709, "s122"),
    (5723, "s123"),
    (5738, "s124"),
    (5752, "s125"),
    (5766, "s126"),
    (5780, "s127"),
    (5795, "s128"),
    (5809, "s129"),
    (5823, "s130"),
    (5838, "s131"),
    (5852, "s132"),
    (5866, "s133"),
    (5881, "s134"),
    (5896, "s135"),
    (5909, "s136"),
    (5923, "s137"),
    (5938, "s138"),
    (5952, "s139"),
    (5966, "s140"),
    (5981, "s141"),
    (5995, "s142"),
    (6009, "s143"),
    (6023, "s144"),
    (6038, "s145"),
    (6052, "s146"),
    (6066, "s147"),
    (6081, "s148"),
    (6095, "s149"),
    (6063, "s150"),
    (6123, "s151"),
    (6138, "s152"),
    (6152, "s153"),
    (6166, "s154"),
    (6181, "s155"),
    (6195, "s156"),
    (6209, "s157"),
    (6223, "s158"),
    (6238, "s159"),
    (6254, "s160"),
    (6266, "s161"),
    (6280, "s162"),
    (6295, "s163"),
    (6309, "s164"),
    (6323, "s165"),
    (6352, "s167"),
    (6366, "s168"),
    (6381, "s169"),
    (6395, "s170"),
    (6423, "s172"),
    (6438, "s173"),
    (6452, "s174"),
    (6420, "s175"),
    (6495, "s177"),
    (6509, "s178"),
    (6523, "s179"),
    (6538, "s180"),
    (6552, "s181"),
    (6566, "s182"),
    (6581, "s183"),
    (6595, "s184"),
    (6611, "s185"),
    (6623, "s186"),
    (6638, "s187"),
    (6652, "s188"),
    (6666, "s189"),
    (6681, "s190"),
    (6695, "s191"),
    (6709, "s192"),
    (6724, "s193"),
    (6692, "s194"),
    (6752, "s195"),
    (6766, "s196"),
    (6781, "s197"),
    (6703, "s198"),
    (6809, "s199"),
    (6777, "s200"),
    (6838, "s201"),
    (6852, "s202"),
    (6866, "s203"),
    (6881, "s204"),
    (6895, "s205"),
    (6909, "s206"),
    (6924, "s207"),
    (6938, "s208"),
    (6952, "s209"),
    (6966, "s210"),
    (6981, "s211"),
    (6995, "s212"),
    (7009, "s213"),
    (7024, "s214"),
    (7038, "s215"),
    (7052, "s216"),
    (7066, "s217"),
    (7081, "s218"),
    (7049, "s219"),
    (7109, "s220"),
];

/// The keys of `scripts/reference/cheats.scenario`'s run of `docs/verification/m5.md`: the
/// test game loaded into the shop as in the race start's run, then DRAW, DROOL, DRIVE and
/// DROP typed, a key every 300 ms.
const CHEATS_KEYS: [(u64, Key); 24] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2190, Key::D),
    (2212, Key::R),
    (2233, Key::A),
    (2254, Key::W),
    (2326, Key::D),
    (2347, Key::R),
    (2369, Key::O),
    (2390, Key::O),
    (2412, Key::L),
    (2483, Key::D),
    (2504, Key::R),
    (2526, Key::I),
    (2547, Key::V),
    (2569, Key::E),
    (2640, Key::D),
    (2662, Key::R),
    (2683, Key::O),
    (2704, Key::P),
];

/// The ticks after which our frame equalled each screenshot of that run: the shop, then the
/// side panel after each word.
const CHEATS_SHOTS: [(u64, &str); 5] = [
    (2140, "k05-space"),
    (2290, "draw"),
    (2447, "drool"),
    (2604, "drive"),
    (2740, "drop"),
];

/// The keys of `scripts/reference/no-sign-up.scenario`'s run of `docs/verification/m5.md`:
/// the race start's way into the sign-up, no choice until every race is full, Enter on the
/// popup saying so, then Enter on each results screen.
const NO_SIGN_UP_KEYS: [(u64, Key); 13] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (4693, Key::Enter),
    (4976, Key::Enter),
    (5190, Key::Enter),
    (5404, Key::Enter),
    (5623, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run: the sign-up filling,
/// the popup, the fade, the three races' pages (none of them the player's), the statistics,
/// and the screen kept after the last key (s036, caught as the hard race's page came in, left
/// out; docs/verification/m5.md).
const NO_SIGN_UP_SHOTS: [(u64, &str); 71] = [
    (3879, "w20"),
    (3882, "w21"),
    (4158, "w22"),
    (4161, "w23"),
    (4298, "w24"),
    (4372, "w25"),
    (4375, "w26"),
    (4379, "s000"),
    (4371, "s001"),
    (4585, "s002"),
    (4602, "s003"),
    (4586, "s004"),
    (4604, "s005"),
    (4656, "s006"),
    (4672, "s007"),
    (4653, "s008"),
    (4710, "s009"),
    (4727, "s010"),
    (132, "s011"),
    (4763, "s012"),
    (4781, "s013"),
    (4799, "s014"),
    (4799, "s015"),
    (4835, "s016"),
    (4832, "s017"),
    (4870, "s018"),
    (4830, "s019"),
    (4838, "s020"),
    (4822, "s021"),
    (4874, "s022"),
    (4826, "s023"),
    (4980, "s024"),
    (4995, "s025"),
    (5011, "s026"),
    (5031, "s027"),
    (5043, "s028"),
    (5033, "s029"),
    (5075, "s030"),
    (5035, "s031"),
    (5040, "s032"),
    (5070, "s033"),
    (5038, "s034"),
    (5174, "s035"),
    (5210, "s037"),
    (5228, "s038"),
    (5245, "s039"),
    (5263, "s040"),
    (5281, "s041"),
    (5299, "s042"),
    (5249, "s043"),
    (5301, "s044"),
    (5284, "s045"),
    (5266, "s046"),
    (5248, "s047"),
    (5297, "s048"),
    (5416, "s049"),
    (5432, "s050"),
    (5414, "s051"),
    (5409, "s052"),
    (5427, "s053"),
    (5429, "s054"),
    (5531, "s055"),
    (5549, "s056"),
    (5545, "s057"),
    (5527, "s058"),
    (5602, "s059"),
    (5622, "s060"),
    (5622, "s061"),
    (5622, "s062"),
    (5623, "s063"),
    (5694, "s064"),
];

/// The keys of `scripts/reference/adversary.scenario`'s run of `docs/verification/m6.md`: the
/// leader's test game loaded into the shop, the Underground Market, its way on to the
/// Adversary's screen, Escape, then Enter on each results screen.
const ADVERSARY_KEYS: [(u64, Key); 13] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2696, Key::Escape),
    (3047, Key::Enter),
    (3261, Key::Enter),
    (3476, Key::Enter),
    (3696, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run: the Adversary's
/// screen, its fade after Escape, the other races' results and the screen kept after them
/// (s000 and s001, as the screen wiped in, left out; docs/verification/m6.md).
const ADVERSARY_SHOTS: [(u64, &str); 98] = [
    (2460, "s002"),
    (2480, "s003"),
    (2461, "s004"),
    (2475, "s005"),
    (2471, "s006"),
    (2470, "s007"),
    (2552, "s008"),
    (2564, "s009"),
    (2550, "s010"),
    (2560, "s011"),
    (2555, "s012"),
    (2623, "s013"),
    (2629, "s014"),
    (2618, "s015"),
    (2632, "s016"),
    (2620, "s017"),
    (2628, "s018"),
    (2709, "s019"),
    (2723, "s020"),
    (2737, "s021"),
    (2752, "s022"),
    (2766, "s023"),
    (2780, "s024"),
    (2795, "s025"),
    (2809, "s026"),
    (2815, "s027"),
    (2835, "s028"),
    (2852, "s029"),
    (2866, "s030"),
    (2859, "s031"),
    (2895, "s032"),
    (2899, "s033"),
    (2885, "s034"),
    (2938, "s035"),
    (2952, "s036"),
    (2966, "s037"),
    (2930, "s038"),
    (2983, "s039"),
    (2969, "s040"),
    (2955, "s041"),
    (2936, "s042"),
    (2985, "s043"),
    (3066, "s044"),
    (3068, "s045"),
    (3087, "s046"),
    (3107, "s047"),
    (3123, "s048"),
    (3069, "s049"),
    (3152, "s050"),
    (3166, "s051"),
    (3171, "s052"),
    (3157, "s053"),
    (3141, "s054"),
    (3163, "s055"),
    (3170, "s056"),
    (3150, "s057"),
    (3269, "s058"),
    (3280, "s059"),
    (3295, "s060"),
    (3309, "s061"),
    (3323, "s062"),
    (3321, "s063"),
    (3352, "s064"),
    (3360, "s065"),
    (3380, "s066"),
    (3395, "s067"),
    (3351, "s068"),
    (3423, "s069"),
    (3438, "s070"),
    (3444, "s071"),
    (3464, "s072"),
    (3482, "s073"),
    (3495, "s074"),
    (3489, "s075"),
    (3523, "s076"),
    (3528, "s077"),
    (3514, "s078"),
    (3498, "s079"),
    (3520, "s080"),
    (3527, "s081"),
    (3491, "s082"),
    (3511, "s083"),
    (3633, "s084"),
    (3652, "s085"),
    (3638, "s086"),
    (3680, "s087"),
    (3628, "s088"),
    (3629, "s089"),
    (3629, "s090"),
    (3629, "s091"),
    (3752, "s092"),
    (3766, "s093"),
    (3780, "s094"),
    (3795, "s095"),
    (3809, "s096"),
    (3823, "s097"),
    (3838, "s098"),
    (3852, "s099"),
];

/// The keys of `scripts/reference/arena.scenario`'s run of `docs/verification/m6.md`: the
/// Adversary run's way in through the Underground Market, Enter on the Adversary's screen;
/// then the Return the race-over box took (held, as the box looks at the keys held), the
/// results' Returns to the hard race's page and to the statistics, at the ticks the original
/// took them (it missed the scenario's first two taps on the box).
const ARENA_KEYS: [(u64, Key); 11] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2161, Key::Enter),
    (2415, Key::Enter),
    (2612, Key::Enter),
    (11256, Key::Enter),
    (11685, Key::Enter),
];
const ARENA_HELD: [Held; 1] = [(11039, Key::Enter, 7)];

/// The ticks after which our frame equalled each screenshot of that run: the Adversary's
/// screen, the preview of the Arena, the race with the Adversary driving its nine laps and the
/// player standing, the flag, the race-over box, the view tilting away and the results.
const ARENA_SHOTS: [(u64, &str); 363] = [
    (2473, "a02"),
    (2509, "a03"),
    (2544, "a04"),
    (2580, "a05"),
    (2646, "p004"),
    (2654, "p005"),
    (2661, "p006"),
    (2668, "p007"),
    (2675, "p008"),
    (2683, "p009"),
    (2690, "p010"),
    (2697, "p011"),
    (2704, "p012"),
    (2711, "p013"),
    (2718, "p014"),
    (2725, "p015"),
    (2733, "p016"),
    (2740, "p017"),
    (2738, "p018"),
    (2746, "r000"),
    (2781, "r001"),
    (2817, "r002"),
    (2852, "r003"),
    (2888, "r004"),
    (2923, "r005"),
    (2959, "r006"),
    (2995, "r007"),
    (3030, "r008"),
    (3066, "r009"),
    (3102, "r010"),
    (3138, "r011"),
    (3173, "r012"),
    (3209, "r013"),
    (3245, "r014"),
    (3280, "r015"),
    (3316, "r016"),
    (3352, "r017"),
    (3388, "r018"),
    (3423, "r019"),
    (3459, "r020"),
    (3494, "r021"),
    (3530, "r022"),
    (3566, "r023"),
    (3601, "r024"),
    (3638, "r025"),
    (3673, "r026"),
    (3709, "r027"),
    (3745, "r028"),
    (3780, "r029"),
    (3816, "r030"),
    (3852, "r031"),
    (3888, "r032"),
    (3923, "r033"),
    (3959, "r034"),
    (3995, "r035"),
    (4031, "r036"),
    (4066, "r037"),
    (4102, "r038"),
    (4138, "r039"),
    (4174, "r040"),
    (4209, "r041"),
    (4245, "r042"),
    (4281, "r043"),
    (4316, "r044"),
    (4352, "r045"),
    (4388, "r046"),
    (4423, "r047"),
    (4459, "r048"),
    (4495, "r049"),
    (4530, "r050"),
    (4566, "r051"),
    (4602, "r052"),
    (4638, "r053"),
    (4673, "r054"),
    (4709, "r055"),
    (4745, "r056"),
    (4780, "r057"),
    (4816, "r058"),
    (4851, "r059"),
    (4888, "r060"),
    (4923, "r061"),
    (4959, "r062"),
    (4994, "r063"),
    (5030, "r064"),
    (5066, "r065"),
    (5101, "r066"),
    (5138, "r067"),
    (5173, "r068"),
    (5209, "r069"),
    (5245, "r070"),
    (5280, "r071"),
    (5316, "r072"),
    (5352, "r073"),
    (5388, "r074"),
    (5423, "r075"),
    (5459, "r076"),
    (5495, "r077"),
    (5531, "r078"),
    (5567, "r079"),
    (5602, "r080"),
    (5638, "r081"),
    (5674, "r082"),
    (5710, "r083"),
    (5745, "r084"),
    (5781, "r085"),
    (5817, "r086"),
    (5852, "r087"),
    (5888, "r088"),
    (5924, "r089"),
    (5959, "r090"),
    (5995, "r091"),
    (6031, "r092"),
    (6066, "r093"),
    (6102, "r094"),
    (6138, "r095"),
    (6174, "r096"),
    (6209, "r097"),
    (6245, "r098"),
    (6281, "r099"),
    (6317, "r100"),
    (6352, "r101"),
    (6388, "r102"),
    (6424, "r103"),
    (6460, "r104"),
    (6495, "r105"),
    (6531, "r106"),
    (6567, "r107"),
    (6602, "r108"),
    (6638, "r109"),
    (6674, "r110"),
    (6709, "r111"),
    (6745, "r112"),
    (6781, "r113"),
    (6816, "r114"),
    (6852, "r115"),
    (6888, "r116"),
    (6924, "r117"),
    (6959, "r118"),
    (6995, "r119"),
    (7031, "r120"),
    (7066, "r121"),
    (7102, "r122"),
    (7138, "r123"),
    (7173, "r124"),
    (7209, "r125"),
    (7245, "r126"),
    (7280, "r127"),
    (7316, "r128"),
    (7352, "r129"),
    (7387, "r130"),
    (7424, "r131"),
    (7459, "r132"),
    (7495, "r133"),
    (7531, "r134"),
    (7566, "r135"),
    (7602, "r136"),
    (7638, "r137"),
    (7674, "r138"),
    (7709, "r139"),
    (7745, "r140"),
    (7781, "r141"),
    (7816, "r142"),
    (7852, "r143"),
    (7888, "r144"),
    (7923, "r145"),
    (7959, "r146"),
    (7995, "r147"),
    (8030, "r148"),
    (8066, "r149"),
    (8102, "r150"),
    (8137, "r151"),
    (8174, "r152"),
    (8209, "r153"),
    (8245, "r154"),
    (8281, "r155"),
    (8316, "r156"),
    (8352, "r157"),
    (8388, "r158"),
    (8424, "r159"),
    (8459, "r160"),
    (8495, "r161"),
    (8531, "r162"),
    (8566, "r163"),
    (8602, "r164"),
    (8638, "r165"),
    (8673, "r166"),
    (8709, "r167"),
    (8745, "r168"),
    (8780, "r169"),
    (8816, "r170"),
    (8852, "r171"),
    (8887, "r172"),
    (8924, "r173"),
    (8959, "r174"),
    (8995, "r175"),
    (9031, "r176"),
    (9066, "r177"),
    (9102, "r178"),
    (9138, "r179"),
    (9174, "r180"),
    (9209, "r181"),
    (9245, "r182"),
    (9281, "r183"),
    (9316, "r184"),
    (9352, "r185"),
    (9388, "r186"),
    (9423, "r187"),
    (9459, "r188"),
    (9495, "r189"),
    (9530, "r190"),
    (9566, "r191"),
    (9602, "r192"),
    (9637, "r193"),
    (9673, "r194"),
    (9710, "r195"),
    (9745, "r196"),
    (9781, "r197"),
    (9817, "r198"),
    (9852, "r199"),
    (9888, "r200"),
    (9924, "r201"),
    (9960, "r202"),
    (9995, "r203"),
    (10031, "r204"),
    (10066, "r205"),
    (10102, "r206"),
    (10138, "r207"),
    (10174, "r208"),
    (10209, "r209"),
    (10245, "r210"),
    (10281, "r211"),
    (10317, "r212"),
    (10352, "r213"),
    (10388, "r214"),
    (10424, "r215"),
    (10459, "r216"),
    (10495, "r217"),
    (10531, "r218"),
    (10567, "r219"),
    (10602, "r220"),
    (10638, "r221"),
    (10673, "e000"),
    (10691, "e001"),
    (10709, "e002"),
    (10727, "e003"),
    (10745, "e004"),
    (10763, "e005"),
    (10781, "e006"),
    (10798, "e007"),
    (10816, "e008"),
    (10834, "e009"),
    (10852, "e010"),
    (10870, "e011"),
    (10888, "e012"),
    (10906, "e013"),
    (10923, "e014"),
    (10941, "e015"),
    (10959, "e016"),
    (10977, "e017"),
    (10995, "e018"),
    (11013, "e019"),
    (11031, "e020"),
    (11042, "e021"),
    (11042, "e022"),
    (11042, "e023"),
    (11042, "e024"),
    (11042, "e025"),
    (11042, "e026"),
    (11042, "e027"),
    (11042, "e028"),
    (10612, "e029"),
    (10612, "e030"),
    (10612, "e031"),
    (10612, "e032"),
    (10612, "e033"),
    (10612, "e034"),
    (10612, "e035"),
    (10612, "e036"),
    (10627, "e037"),
    (10645, "e038"),
    (10663, "e039"),
    (10681, "e040"),
    (10699, "e041"),
    (10717, "e042"),
    (10735, "e043"),
    (10752, "e044"),
    (10770, "e045"),
    (10788, "e046"),
    (10806, "e047"),
    (10824, "e048"),
    (10842, "e049"),
    (10860, "e050"),
    (10877, "e051"),
    (10895, "e052"),
    (10913, "e053"),
    (10931, "e054"),
    (10949, "e055"),
    (10967, "e056"),
    (10985, "e057"),
    (11002, "e058"),
    (11020, "e059"),
    (11038, "e060"),
    (11056, "e061"),
    (11074, "e062"),
    (11092, "e063"),
    (11110, "e064"),
    (11127, "e065"),
    (11143, "e066"),
    (11201, "e069"),
    (11218, "e070"),
    (11236, "e071"),
    (11271, "e073"),
    (11289, "e074"),
    (11307, "e075"),
    (11325, "e076"),
    (11343, "e077"),
    (11361, "e078"),
    (11370, "e079"),
    (11396, "e080"),
    (11414, "e081"),
    (11418, "e082"),
    (11434, "e083"),
    (11434, "e084"),
    (11466, "e085"),
    (11503, "e086"),
    (11521, "e087"),
    (11539, "e088"),
    (11531, "e089"),
    (11575, "e090"),
    (11593, "e091"),
    (11611, "e092"),
    (11628, "e093"),
    (11680, "e094"),
    (11662, "e095"),
    (11683, "e096"),
    (11698, "e097"),
    (11734, "e099"),
    (11752, "e100"),
    (11743, "e101"),
    (11787, "e102"),
    (11805, "e103"),
    (11823, "e104"),
    (11841, "e105"),
    (11891, "e106"),
    (11877, "e107"),
    (11894, "e108"),
    (11912, "e109"),
    (11930, "e110"),
    (11938, "e111"),
    (11966, "e112"),
    (11984, "e113"),
    (12002, "e114"),
    (12019, "e115"),
    (12019, "e116"),
    (12055, "e117"),
    (12073, "e118"),
    (12091, "e119"),
    (12109, "e120"),
    (12100, "e121"),
    (12144, "e122"),
    (12162, "e123"),
    (12180, "e124"),
    (12198, "e125"),
];

/// The keys of `scripts/reference/statistics.scenario`'s run of `docs/verification/m5.md`: the
/// test game loaded into the shop as in the race start's run, Escape back to the Start Racing
/// menu, Up to its statistics row, Enter, and Enter on the statistics.
const STATISTICS_KEYS: [(u64, Key); 10] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1949, Key::Enter),
    (2033, Key::Space),
    (2162, Key::Escape),
    (2261, Key::Up),
    (2312, Key::Enter),
    (2668, Key::Enter),
];

/// The ticks after which our frame equalled each screenshot of that run: the menu fading out,
/// the statistics in, their wait, out, and the menu back in (seven shots whose pulsing line is
/// a step off left out; docs/verification/m5.md).
const STATISTICS_SHOTS: [(u64, &str); 46] = [
    (2325, "s000"),
    (2339, "s001"),
    (2354, "s002"),
    (2426, "s007"),
    (2418, "s008"),
    (2455, "s009"),
    (2457, "s010"),
    (2443, "s011"),
    (2428, "s012"),
    (2448, "s013"),
    (2458, "s014"),
    (2420, "s015"),
    (2555, "s016"),
    (2569, "s017"),
    (2581, "s018"),
    (2563, "s019"),
    (2552, "s020"),
    (2626, "s021"),
    (2626, "s022"),
    (2645, "s023"),
    (2664, "s024"),
    (2365, "s028"),
    (2365, "s029"),
    (2365, "s030"),
    (2365, "s031"),
    (2781, "s032"),
    (2795, "s033"),
    (2810, "s034"),
    (2823, "s035"),
    (2837, "s036"),
    (2853, "s037"),
    (2867, "s038"),
    (2881, "s039"),
    (2895, "s040"),
    (2909, "s041"),
    (2923, "s042"),
    (2937, "s043"),
    (2953, "s044"),
    (2967, "s045"),
    (2981, "s046"),
    (2995, "s047"),
    (3009, "s048"),
    (3023, "s049"),
    (3037, "s050"),
    (3053, "s051"),
    (3067, "s052"),
];

/// The keys of `scripts/reference/shop-welcome.scenario`'s run of `docs/verification/m5.md`
/// after the new game run's: space ending the sign-up's wait, then (each held) Enter on the
/// first race's welcome box, Escape and Y abandoning the race 2.49 s into it as in the
/// original, and Enter on each results screen, on the shop's welcome, on its last place's popup
/// and on into the Underground Market.
const SHOP_WELCOME_KEYS: [(u64, Key); 8] = [
    (3334, Key::Space),
    (4977, Key::Enter),
    (5191, Key::Enter),
    (5405, Key::Enter),
    (5604, Key::Enter),
    (5818, Key::Enter),
    (6076, Key::Enter),
    (6290, Key::Enter),
];
const SHOP_WELCOME_HELD: [Held; 3] = [
    (4120, Key::Enter, 7),
    (4549, Key::Escape, 7),
    (4692, Key::Y, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the race, its
/// welcome box flying in, the pause, the results and the statistics, the shop fading in with
/// the welcome to it and its cursor, the last place's popup after it, the shop drawn again
/// with its border, and the Underground Market after its first visit (the Enters on the
/// market's way on where its shots after them put them).
const SHOP_WELCOME_SHOTS: [(u64, &str); 106] = [
    (3540, "race-1"),
    (3824, "race-2"),
    (3861, "help"),
    (4184, "race-3"),
    (4334, "race-4"),
    (4415, "race-5"),
    (4619, "paused"),
    (4705, "w000"),
    (4720, "w001"),
    (4734, "w002"),
    (4748, "w003"),
    (4764, "w004"),
    (4778, "w005"),
    (4792, "w006"),
    (4858, "w011"),
    (4857, "w012"),
    (4877, "w013"),
    (4897, "w014"),
    (4915, "w015"),
    (4930, "w016"),
    (4944, "w017"),
    (4942, "w018"),
    (4906, "w019"),
    (4981, "w020"),
    (5001, "w021"),
    (5015, "w022"),
    (5030, "w023"),
    (5044, "w024"),
    (5058, "w025"),
    (5065, "w026"),
    (5051, "w027"),
    (5033, "w028"),
    (5056, "w029"),
    (5062, "w030"),
    (5062, "w031"),
    (5048, "w032"),
    (5173, "w033"),
    (5188, "w034"),
    (5200, "w035"),
    (5208, "w036"),
    (5195, "w037"),
    (5257, "w039"),
    (5272, "w040"),
    (5272, "w041"),
    (5258, "w042"),
    (5244, "w043"),
    (5261, "w044"),
    (5275, "w045"),
    (5269, "w046"),
    (5254, "w047"),
    (5250, "w048"),
    (5266, "w049"),
    (5461, "w053"),
    (5476, "w054"),
    (5490, "w055"),
    (5504, "w056"),
    (5517, "w057"),
    (5533, "w058"),
    (5547, "w059"),
    (5561, "w060"),
    (5575, "w061"),
    (5589, "w062"),
    (5603, "w063"),
    (5519, "w064"),
    (5633, "w065"),
    (5647, "w066"),
    (5661, "w067"),
    (5675, "w068"),
    (5689, "w069"),
    (5703, "w070"),
    (5717, "w071"),
    (5633, "w072"),
    (5647, "w073"),
    (5661, "w074"),
    (5775, "w075"),
    (5789, "w076"),
    (5805, "w077"),
    (5819, "w078"),
    (5877, "w080"),
    (5891, "w081"),
    (5906, "w082"),
    (5920, "w083"),
    (5934, "w084"),
    (5948, "w085"),
    (5963, "w086"),
    (5977, "w087"),
    (5991, "w088"),
    (6006, "w089"),
    (6020, "w090"),
    (6034, "w091"),
    (6334, "w112"),
    (6349, "w113"),
    (6363, "w114"),
    (6377, "w115"),
    (6391, "w116"),
    (6406, "w117"),
    (6420, "w118"),
    (6296, "w119"),
    (6448, "w120"),
    (6463, "w121"),
    (6477, "w122"),
    (6307, "w123"),
    (6534, "w126"),
    (6549, "w127"),
    (6563, "w128"),
    (6577, "w129"),
];

/// The ticks after which our frame equalled each screenshot of `scripts/reference/opponents.scenario`'s
/// run of `docs/verification/m5.md` (no keys held: the player stands while the opponents race
/// three laps round it).
const OPPONENTS_SHOTS: [(u64, &str); 126] = [
    (3360, "o000"),
    (3395, "o001"),
    (3431, "o002"),
    (3467, "o003"),
    (3502, "o004"),
    (3538, "o005"),
    (3574, "o006"),
    (3605, "o007"),
    (3642, "o008"),
    (3681, "o009"),
    (3716, "o010"),
    (3752, "o011"),
    (3780, "o012"),
    (3815, "o013"),
    (3850, "o014"),
    (3885, "o015"),
    (3920, "o016"),
    (3965, "o017"),
    (3999, "o018"),
    (4037, "o019"),
    (4073, "o020"),
    (4109, "o021"),
    (4140, "o022"),
    (4167, "o023"),
    (4200, "o024"),
    (4235, "o025"),
    (4270, "o026"),
    (4305, "o027"),
    (4340, "o028"),
    (4387, "o029"),
    (4410, "o030"),
    (4445, "o031"),
    (4480, "o032"),
    (4515, "o033"),
    (4550, "o034"),
    (4585, "o035"),
    (4645, "o036"),
    (4655, "o037"),
    (4690, "o038"),
    (4725, "o039"),
    (4760, "o040"),
    (4795, "o041"),
    (4830, "o042"),
    (4865, "o043"),
    (4900, "o044"),
    (4935, "o045"),
    (4970, "o046"),
    (5005, "o047"),
    (5040, "o048"),
    (5075, "o049"),
    (5110, "o050"),
    (5145, "o051"),
    (5180, "o052"),
    (5246, "o053"),
    (5250, "o054"),
    (5323, "o055"),
    (5358, "o056"),
    (5395, "o057"),
    (5423, "o058"),
    (5425, "o059"),
    (5493, "o060"),
    (5495, "o061"),
    (5533, "o062"),
    (5592, "o063"),
    (5603, "o064"),
    (5632, "o065"),
    (5712, "o066"),
    (5740, "o067"),
    (5740, "o068"),
    (5824, "o069"),
    (5860, "o070"),
    (5895, "o071"),
    (5931, "o072"),
    (5967, "o073"),
    (6001, "o074"),
    (6038, "o075"),
    (6074, "o076"),
    (6110, "o077"),
    (6145, "o078"),
    (6181, "o079"),
    (6217, "o080"),
    (6252, "o081"),
    (6256, "o082"),
    (6265, "o083"),
    (6360, "o084"),
    (6395, "o085"),
    (6431, "o086"),
    (6467, "o087"),
    (6485, "o088"),
    (6485, "o089"),
    (6510, "o090"),
    (6545, "o091"),
    (6580, "o092"),
    (6615, "o093"),
    (6650, "o094"),
    (6685, "o095"),
    (6720, "o096"),
    (6755, "o097"),
    (6790, "o098"),
    (6825, "o099"),
    (6860, "o100"),
    (6895, "o101"),
    (6930, "o102"),
    (6965, "o103"),
    (7000, "o104"),
    (7035, "o105"),
    (7070, "o106"),
    (7105, "o107"),
    (7140, "o108"),
    (7175, "o109"),
    (7210, "o110"),
    (7245, "o111"),
    (7280, "o112"),
    (7315, "o113"),
    (7350, "o114"),
    (7385, "o115"),
    (7457, "o116"),
    (7457, "o117"),
    (7490, "o118"),
    (7527, "o119"),
    (7560, "o120"),
    (7597, "o121"),
    (7626, "o122"),
    (7667, "o123"),
    (7696, "o124"),
    (7737, "o125"),
];

/// The keys held in `scripts/reference/effect.scenario`'s run of `docs/verification/m5.md`
/// (`--no-ai --drive`), as the original sampled them: Up held 14 ticks before the first
/// power-ups (so the effect power-up lies after the first U-turn), then the car driven over it.
const EFFECT_HELD: [Held; 33] = [
    (3488, Key::Up, 14),
    (3669, Key::Up, 292),
    (3670, Key::Left, 5),
    (3701, Key::Right, 3),
    (3708, Key::Right, 3),
    (3720, Key::Right, 2),
    (3733, Key::Right, 2),
    (3750, Key::Right, 2),
    (3763, Key::Right, 2),
    (3782, Key::Right, 2),
    (3787, Key::Right, 2),
    (3795, Key::Right, 2),
    (3801, Key::Right, 2),
    (3807, Key::Right, 34),
    (3843, Key::Right, 6),
    (3851, Key::Right, 15),
    (3869, Key::Right, 5),
    (3877, Key::Right, 2),
    (3882, Key::Right, 3),
    (3889, Key::Right, 2),
    (3895, Key::Right, 2),
    (3903, Key::Right, 2),
    (3924, Key::Left, 4),
    (3931, Key::Left, 10),
    (3943, Key::Left, 72),
    (3986, Key::Up, 135),
    (4020, Key::Left, 2),
    (4031, Key::Left, 2),
    (4043, Key::Left, 2),
    (4091, Key::Left, 2),
    (4103, Key::Left, 2),
    (4109, Key::Left, 3),
    (4116, Key::Left, 3),
];

/// The ticks after which our frame equalled each screenshot of that run: the drive, the effect
/// power-up taken at frame 886 and the view wavering for its 560 ticks, then still again.
const EFFECT_SHOTS: [(u64, &str); 131] = [
    (3363, "e000"),
    (3394, "e001"),
    (3430, "e002"),
    (3466, "e003"),
    (3503, "e004"),
    (3537, "e005"),
    (3573, "e006"),
    (3608, "e007"),
    (3644, "e008"),
    (3680, "e009"),
    (3715, "e010"),
    (3751, "e011"),
    (3787, "e012"),
    (3823, "e013"),
    (3858, "e014"),
    (3894, "w000"),
    (3901, "w001"),
    (3908, "w002"),
    (3916, "w003"),
    (3923, "w004"),
    (3930, "w005"),
    (3937, "w006"),
    (3944, "w007"),
    (3951, "w008"),
    (3958, "w009"),
    (3965, "w010"),
    (3973, "w011"),
    (3980, "w012"),
    (3987, "w013"),
    (3994, "w014"),
    (4001, "w015"),
    (4008, "w016"),
    (4015, "w017"),
    (4023, "w018"),
    (4030, "w019"),
    (4037, "w020"),
    (4044, "w021"),
    (4051, "w022"),
    (4058, "w023"),
    (4065, "w024"),
    (4073, "w025"),
    (4080, "w026"),
    (4087, "w027"),
    (4094, "w028"),
    (4101, "w029"),
    (4108, "w030"),
    (4115, "w031"),
    (4123, "w032"),
    (4130, "w033"),
    (4137, "w034"),
    (4144, "w035"),
    (4151, "w036"),
    (4158, "w037"),
    (4166, "w038"),
    (4173, "w039"),
    (4180, "w040"),
    (4187, "w041"),
    (4194, "w042"),
    (4201, "w043"),
    (4208, "w044"),
    (4216, "w045"),
    (4223, "w046"),
    (4230, "w047"),
    (4237, "w048"),
    (4244, "w049"),
    (4251, "w050"),
    (4258, "w051"),
    (4266, "w052"),
    (4273, "w053"),
    (4280, "w054"),
    (4287, "w055"),
    (4294, "w056"),
    (4301, "w057"),
    (4308, "w058"),
    (4316, "w059"),
    (4323, "w060"),
    (4330, "w061"),
    (4337, "w062"),
    (4344, "w063"),
    (4351, "w064"),
    (4358, "w065"),
    (4366, "w066"),
    (4373, "w067"),
    (4380, "w068"),
    (4387, "w069"),
    (4394, "w070"),
    (4401, "w071"),
    (4408, "w072"),
    (4415, "w073"),
    (4423, "w074"),
    (4430, "w075"),
    (4437, "w076"),
    (4444, "w077"),
    (4451, "w078"),
    (4458, "w079"),
    (4465, "w080"),
    (4473, "w081"),
    (4480, "w082"),
    (4487, "w083"),
    (4494, "w084"),
    (4501, "w085"),
    (4508, "w086"),
    (4515, "w087"),
    (4523, "w088"),
    (4530, "w089"),
    (4537, "w090"),
    (4544, "w091"),
    (4551, "w092"),
    (4558, "w093"),
    (4566, "w094"),
    (4573, "w095"),
    (4580, "w096"),
    (4587, "w097"),
    (4594, "w098"),
    (4601, "w099"),
    (4608, "w100"),
    (4616, "w101"),
    (4623, "w102"),
    (4630, "w103"),
    (4637, "w104"),
    (4644, "w105"),
    (4651, "w106"),
    (4658, "w107"),
    (4665, "w108"),
    (4672, "w109"),
    (4679, "w110"),
    (4686, "w111"),
    (4693, "w112"),
    (4700, "w113"),
    (4707, "w114"),
    (4714, "w115"),
];

/// The keys held in `scripts/reference/race-keys.scenario`'s run of `docs/verification/m5.md`
/// (`--no-ai --drive`): the car's keys as the original sampled them, and TAB, F5, F4, P and
/// Enter at the ticks whose frames equal the run's screenshots.
const RACE_KEYS_HELD: [Held; 36] = [
    (3395, Key::Tab, 7),
    (3459, Key::F5, 7),
    (3488, Key::F5, 7),
    (3509, Key::Up, 317),
    (3510, Key::Left, 9),
    (3528, Key::Right, 2),
    (3537, Key::Right, 2),
    (3549, Key::Right, 2),
    (3560, Key::Right, 2),
    (3578, Key::Right, 2),
    (3600, Key::Right, 2),
    (3620, Key::Right, 2),
    (3642, Key::Right, 3),
    (3651, Key::Right, 3),
    (3662, Key::Right, 2),
    (3669, Key::Right, 36),
    (3707, Key::Right, 7),
    (3716, Key::Right, 11),
    (3729, Key::Right, 6),
    (3739, Key::Right, 3),
    (3745, Key::Right, 2),
    (3750, Key::Right, 2),
    (3757, Key::Right, 3),
    (3768, Key::Right, 2),
    (3773, Key::F4, 7),
    (3784, Key::Left, 3),
    (3790, Key::Left, 4),
    (3796, Key::Left, 8),
    (3807, Key::Left, 65),
    (3816, Key::F4, 7),
    (3829, Key::Up, 2),
    (3844, Key::Up, 34),
    (3866, Key::Tab, 7),
    (3930, Key::Tab, 7),
    (4002, Key::P, 7),
    (4145, Key::Enter, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the status bar sliding
/// away, the small board, the shadows off and on, the drive, the scene's pictures off and on,
/// the status bar back and away again, and the game paused in the middle of the wider view.
const RACE_KEYS_SHOTS: [(u64, &str); 126] = [
    (3387, "k000"),
    (3396, "k001"),
    (3403, "k002"),
    (3409, "k003"),
    (3416, "k004"),
    (3423, "k005"),
    (3430, "k006"),
    (3437, "k007"),
    (3444, "k008"),
    (3451, "k009"),
    (3459, "k010"),
    (3466, "k011"),
    (3473, "k012"),
    (3480, "k013"),
    (3490, "k014"),
    (3497, "k015"),
    (3503, "k016"),
    (3510, "k017"),
    (3516, "k018"),
    (3523, "k019"),
    (3530, "k020"),
    (3537, "k021"),
    (3544, "k022"),
    (3552, "k023"),
    (3559, "k024"),
    (3566, "k025"),
    (3573, "k026"),
    (3580, "k027"),
    (3587, "k028"),
    (3595, "k029"),
    (3602, "k030"),
    (3609, "k031"),
    (3616, "k032"),
    (3623, "k033"),
    (3630, "k034"),
    (3637, "k035"),
    (3644, "k036"),
    (3652, "k037"),
    (3659, "k038"),
    (3666, "k039"),
    (3673, "k040"),
    (3680, "k041"),
    (3687, "k042"),
    (3695, "k043"),
    (3702, "k044"),
    (3709, "k045"),
    (3716, "k046"),
    (3723, "k047"),
    (3731, "k048"),
    (3737, "k049"),
    (3744, "k050"),
    (3752, "k051"),
    (3759, "k052"),
    (3766, "k053"),
    (3775, "k054"),
    (3782, "k055"),
    (3787, "k056"),
    (3794, "k057"),
    (3802, "k058"),
    (3809, "k059"),
    (3817, "k060"),
    (3825, "k061"),
    (3830, "k062"),
    (3837, "k063"),
    (3845, "k064"),
    (3852, "k065"),
    (3859, "k066"),
    (3867, "k067"),
    (3875, "k068"),
    (3881, "k069"),
    (3887, "k070"),
    (3895, "k071"),
    (3902, "k072"),
    (3909, "k073"),
    (3916, "k074"),
    (3923, "k075"),
    (3931, "k076"),
    (3939, "k077"),
    (3944, "k078"),
    (3952, "k079"),
    (3959, "k080"),
    (3966, "k081"),
    (3973, "k082"),
    (3980, "k083"),
    (3987, "k084"),
    (3994, "k085"),
    (4002, "k086"),
    (4010, "k087"),
    (4016, "k088"),
    (4023, "k089"),
    (4030, "k090"),
    (4037, "k091"),
    (4044, "k092"),
    (4052, "k093"),
    (4059, "k094"),
    (4066, "k095"),
    (4073, "k096"),
    (4080, "k097"),
    (4087, "k098"),
    (4095, "k099"),
    (4102, "k100"),
    (4109, "k101"),
    (4116, "k102"),
    (4123, "k103"),
    (4130, "k104"),
    (4137, "k105"),
    (4144, "k106"),
    (4152, "k107"),
    (4158, "k108"),
    (4165, "k109"),
    (4172, "k110"),
    (4179, "k111"),
    (4186, "k112"),
    (4194, "k113"),
    (4201, "k114"),
    (4208, "k115"),
    (4215, "k116"),
    (4222, "k117"),
    (4229, "k118"),
    (4236, "k119"),
    (4243, "k120"),
    (4250, "k121"),
    (4257, "k122"),
    (4264, "k123"),
    (4271, "k124"),
    (4279, "k125"),
];

/// The keys held in `scripts/reference/lap.scenario`'s run of `docs/verification/m5.md`
/// (the test game without weapons, `--no-ai --drive`): the car's keys for a whole lap as the
/// original sampled them, and TAB twice after the lap.
const LAP_HELD: [Held; 215] = [
    (3199, Key::Up, 329),
    (3200, Key::Left, 12),
    (3231, Key::Right, 2),
    (3239, Key::Right, 2),
    (3250, Key::Right, 2),
    (3267, Key::Right, 2),
    (3287, Key::Right, 2),
    (3311, Key::Right, 2),
    (3333, Key::Right, 2),
    (3341, Key::Right, 2),
    (3346, Key::Right, 2),
    (3354, Key::Right, 2),
    (3361, Key::Right, 6),
    (3369, Key::Right, 31),
    (3402, Key::Right, 10),
    (3414, Key::Right, 9),
    (3425, Key::Right, 4),
    (3432, Key::Right, 3),
    (3439, Key::Right, 3),
    (3446, Key::Right, 2),
    (3452, Key::Right, 2),
    (3459, Key::Right, 2),
    (3484, Key::Left, 2),
    (3489, Key::Left, 5),
    (3497, Key::Left, 4),
    (3503, Key::Left, 3),
    (3508, Key::Left, 62),
    (3540, Key::Up, 1083),
    (3579, Key::Left, 2),
    (3599, Key::Left, 2),
    (3649, Key::Left, 3),
    (3656, Key::Left, 3),
    (3663, Key::Left, 2),
    (3668, Key::Left, 4),
    (3676, Key::Left, 4),
    (3684, Key::Left, 2),
    (3689, Key::Left, 6),
    (3699, Key::Left, 4),
    (3705, Key::Left, 3),
    (3711, Key::Left, 2),
    (3717, Key::Left, 2),
    (3723, Key::Left, 2),
    (3731, Key::Left, 2),
    (3741, Key::Left, 2),
    (3788, Key::Right, 2),
    (3819, Key::Left, 3),
    (3826, Key::Left, 2),
    (3831, Key::Left, 3),
    (3839, Key::Left, 3),
    (3847, Key::Left, 3),
    (3854, Key::Left, 2),
    (3862, Key::Left, 2),
    (3869, Key::Left, 2),
    (3878, Key::Left, 2),
    (3894, Key::Left, 2),
    (3919, Key::Right, 2),
    (3924, Key::Right, 2),
    (3930, Key::Right, 3),
    (3937, Key::Right, 2),
    (3968, Key::Left, 2),
    (3973, Key::Left, 3),
    (3980, Key::Left, 4),
    (3987, Key::Left, 3),
    (3995, Key::Left, 3),
    (4001, Key::Left, 4),
    (4010, Key::Left, 2),
    (4017, Key::Left, 2),
    (4025, Key::Left, 2),
    (4034, Key::Left, 2),
    (4041, Key::Left, 2),
    (4054, Key::Left, 2),
    (4062, Key::Left, 2),
    (4070, Key::Left, 2),
    (4076, Key::Left, 2),
    (4082, Key::Left, 3),
    (4087, Key::Left, 5),
    (4095, Key::Left, 3),
    (4101, Key::Left, 3),
    (4106, Key::Left, 2),
    (4114, Key::Left, 2),
    (4120, Key::Left, 2),
    (4142, Key::Right, 3),
    (4147, Key::Right, 21),
    (4182, Key::Right, 2),
    (4188, Key::Right, 2),
    (4193, Key::Right, 4),
    (4199, Key::Right, 32),
    (4234, Key::Right, 3),
    (4240, Key::Right, 2),
    (4254, Key::Left, 5),
    (4262, Key::Left, 9),
    (4273, Key::Left, 4),
    (4281, Key::Left, 2),
    (4288, Key::Left, 2),
    (4304, Key::Left, 2),
    (4321, Key::Right, 2),
    (4328, Key::Right, 3),
    (4336, Key::Right, 2),
    (4369, Key::Left, 3),
    (4375, Key::Left, 5),
    (4383, Key::Left, 7),
    (4392, Key::Left, 21),
    (4415, Key::Left, 11),
    (4428, Key::Left, 8),
    (4439, Key::Left, 2),
    (4444, Key::Left, 2),
    (4478, Key::Right, 2),
    (4509, Key::Left, 2),
    (4514, Key::Left, 2),
    (4521, Key::Left, 3),
    (4529, Key::Left, 4),
    (4536, Key::Left, 3),
    (4541, Key::Left, 3),
    (4547, Key::Left, 4),
    (4554, Key::Left, 3),
    (4562, Key::Left, 2),
    (4568, Key::Left, 2),
    (4594, Key::Right, 5),
    (4601, Key::Right, 70),
    (4652, Key::Up, 654),
    (4680, Key::Right, 2),
    (4709, Key::Left, 3),
    (4715, Key::Left, 5),
    (4722, Key::Left, 5),
    (4729, Key::Left, 8),
    (4739, Key::Left, 12),
    (4753, Key::Left, 14),
    (4769, Key::Left, 7),
    (4778, Key::Left, 4),
    (4784, Key::Left, 3),
    (4789, Key::Left, 5),
    (4797, Key::Left, 3),
    (4803, Key::Left, 3),
    (4809, Key::Left, 3),
    (4816, Key::Left, 3),
    (4822, Key::Left, 2),
    (4828, Key::Left, 3),
    (4835, Key::Left, 2),
    (4847, Key::Left, 5),
    (4854, Key::Left, 4),
    (4867, Key::Left, 2),
    (4910, Key::Right, 3),
    (4916, Key::Right, 6),
    (4927, Key::Right, 3),
    (4933, Key::Right, 2),
    (4938, Key::Right, 2),
    (4944, Key::Right, 2),
    (4950, Key::Right, 2),
    (4973, Key::Right, 3),
    (4979, Key::Right, 2),
    (4983, Key::Right, 2),
    (4987, Key::Right, 2),
    (4993, Key::Right, 2),
    (5004, Key::Right, 2),
    (5040, Key::Left, 3),
    (5058, Key::Left, 3),
    (5068, Key::Left, 2),
    (5075, Key::Left, 2),
    (5089, Key::Left, 2),
    (5099, Key::Left, 2),
    (5106, Key::Left, 2),
    (5114, Key::Left, 2),
    (5119, Key::Left, 2),
    (5124, Key::Left, 2),
    (5129, Key::Left, 2),
    (5134, Key::Left, 2),
    (5139, Key::Left, 4),
    (5146, Key::Left, 4),
    (5154, Key::Left, 4),
    (5160, Key::Left, 3),
    (5165, Key::Left, 3),
    (5170, Key::Left, 5),
    (5177, Key::Left, 5),
    (5184, Key::Left, 3),
    (5189, Key::Left, 5),
    (5196, Key::Left, 4),
    (5203, Key::Left, 3),
    (5208, Key::Left, 4),
    (5216, Key::Left, 4),
    (5224, Key::Left, 3),
    (5231, Key::Left, 3),
    (5237, Key::Left, 3),
    (5245, Key::Left, 2),
    (5272, Key::Right, 5),
    (5280, Key::Right, 8),
    (5290, Key::Right, 62),
    (5327, Key::Up, 398),
    (5362, Key::Right, 2),
    (5373, Key::Right, 3),
    (5380, Key::Right, 2),
    (5392, Key::Left, 2),
    (5399, Key::Left, 2),
    (5410, Key::Left, 2),
    (5435, Key::Left, 2),
    (5533, Key::Right, 2),
    (5545, Key::Right, 2),
    (5554, Key::Right, 2),
    (5561, Key::Right, 2),
    (5565, Key::Right, 3),
    (5571, Key::Right, 2),
    (5576, Key::Right, 4),
    (5583, Key::Right, 4),
    (5590, Key::Right, 49),
    (5631, Key::Tab, 7),
    (5642, Key::Right, 5),
    (5650, Key::Right, 5),
    (5658, Key::Right, 4),
    (5666, Key::Right, 2),
    (5673, Key::Right, 2),
    (5689, Key::Left, 4),
    (5695, Key::Left, 9),
    (5707, Key::Left, 2),
    (5717, Key::Left, 2),
    (5722, Key::Left, 4),
    (5788, Key::Tab, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the HUD's clock through
/// the lap, the lap's time after it, the status bar sliding away with the small board showing
/// the lap's time and then the new lap's clock, and the bar back.
const LAP_SHOTS: [(u64, &str); 81] = [
    (3359, "c000"),
    (3430, "c001"),
    (3502, "c002"),
    (3573, "c003"),
    (3645, "c004"),
    (3716, "c005"),
    (3788, "c006"),
    (3859, "c007"),
    (3931, "c008"),
    (4002, "c009"),
    (4073, "c010"),
    (4145, "c011"),
    (4216, "c012"),
    (4288, "c013"),
    (4359, "c014"),
    (4431, "c015"),
    (4502, "c016"),
    (4573, "c017"),
    (4645, "c018"),
    (4716, "c019"),
    (4788, "c020"),
    (4859, "c021"),
    (4930, "c022"),
    (5002, "c023"),
    (5074, "c024"),
    (5145, "c025"),
    (5216, "c026"),
    (5288, "c027"),
    (5359, "c028"),
    (5431, "c029"),
    (5502, "t000"),
    (5509, "t001"),
    (5516, "t002"),
    (5523, "t003"),
    (5531, "t004"),
    (5538, "t005"),
    (5545, "t006"),
    (5552, "t007"),
    (5559, "t008"),
    (5566, "t009"),
    (5574, "t010"),
    (5581, "t011"),
    (5588, "t012"),
    (5595, "t013"),
    (5602, "t014"),
    (5609, "t015"),
    (5616, "t016"),
    (5623, "t017"),
    (5632, "t018"),
    (5639, "t019"),
    (5645, "t020"),
    (5652, "t021"),
    (5659, "t022"),
    (5666, "t023"),
    (5674, "t024"),
    (5681, "t025"),
    (5688, "t026"),
    (5695, "t027"),
    (5702, "t028"),
    (5709, "t029"),
    (5716, "t030"),
    (5723, "t031"),
    (5731, "t032"),
    (5738, "t033"),
    (5745, "t034"),
    (5752, "t035"),
    (5759, "t036"),
    (5767, "t037"),
    (5773, "t038"),
    (5781, "t039"),
    (5789, "t040"),
    (5797, "t041"),
    (5802, "t042"),
    (5809, "t043"),
    (5816, "t044"),
    (5823, "t045"),
    (5831, "t046"),
    (5838, "t047"),
    (5845, "t048"),
    (5852, "t049"),
    (5859, "t050"),
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

/// The keys held in `scripts/reference/tab-abort.scenario`'s run of `docs/verification/m5.md`
/// (`--no-ai`), at the ticks whose frames equal its screenshots: TAB once the race is on,
/// Escape, Y.
const TAB_ABORT_HELD: [Held; 3] = [
    (3359, Key::Tab, 7),
    (3430, Key::Escape, 7),
    (3609, Key::Y, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the status bar away,
/// the box over the wider view, and the race's last frame spinning away.
const TAB_ABORT_SHOTS: [(u64, &str); 53] = [
    (3417, "hidden"),
    (3428, "last"),
    (3596, "box"),
    (3616, "s00"),
    (3619, "s01"),
    (3622, "s02"),
    (3626, "s03"),
    (3629, "s04"),
    (3633, "s05"),
    (3637, "s06"),
    (3640, "s07"),
    (3644, "s08"),
    (3647, "s09"),
    (3651, "s10"),
    (3654, "s11"),
    (3658, "s12"),
    (3662, "s13"),
    (3665, "s14"),
    (3669, "s15"),
    (3673, "s16"),
    (3677, "s17"),
    (3680, "s18"),
    (3684, "s19"),
    (3688, "s20"),
    (3691, "s21"),
    (3695, "s22"),
    (3698, "s23"),
    (3702, "s24"),
    (3705, "s25"),
    (3709, "s26"),
    (3710, "s27"),
    (3713, "s28"),
    (3717, "s29"),
    (3720, "s30"),
    (3724, "s31"),
    (3727, "s32"),
    (3731, "s33"),
    (3735, "s34"),
    (3738, "s35"),
    (3742, "s36"),
    (3745, "s37"),
    (3749, "s38"),
    (3753, "s39"),
    (3756, "s40"),
    (3760, "s41"),
    (3763, "s42"),
    (3767, "s43"),
    (3770, "s44"),
    (3774, "s45"),
    (3777, "s46"),
    (3781, "s47"),
    (3785, "s48"),
    (3788, "s49"),
];

/// The keys held in `scripts/reference/abort-early.scenario`'s run: Escape held through the
/// race's load, so the loop's first pass pauses before the intro, then Y.
const ABORT_EARLY_HELD: [Held; 2] = [(2990, Key::Escape, 40), (3213, Key::Y, 7)];

/// The ticks after which our frame equalled each screenshot of that run: the preview's fade,
/// the box flying in and apart over the black palette, the intro run all the same and the view
/// tilting away.
const ABORT_EARLY_SHOTS: [(u64, &str); 74] = [
    (2974, "e11"),
    (2981, "e12"),
    (2988, "e13"),
    (2996, "e14"),
    (3003, "e15"),
    (3010, "e16"),
    (3017, "e17"),
    (3024, "e18"),
    (3031, "e19"),
    (3038, "e20"),
    (3045, "e21"),
    (3052, "e22"),
    (3059, "e23"),
    (3066, "e24"),
    (3073, "e25"),
    (3080, "e26"),
    (3087, "e27"),
    (3094, "e28"),
    (3101, "e29"),
    (3108, "e30"),
    (3115, "e31"),
    (3122, "e32"),
    (3129, "e33"),
    (3136, "e34"),
    (3143, "e35"),
    (3150, "e36"),
    (3157, "e37"),
    (3164, "e38"),
    (3171, "e39"),
    (3178, "e40"),
    (3185, "e41"),
    (3192, "e42"),
    (3199, "e43"),
    (3206, "e44"),
    (3213, "e45"),
    (3220, "e46"),
    (3227, "e47"),
    (3234, "e48"),
    (3241, "e49"),
    (3248, "e50"),
    (3255, "e51"),
    (3262, "e52"),
    (3269, "e53"),
    (3276, "e54"),
    (3283, "e55"),
    (3290, "e56"),
    (3298, "e57"),
    (3305, "e58"),
    (3312, "e59"),
    (3319, "e60"),
    (3326, "e61"),
    (3333, "e62"),
    (3340, "e63"),
    (3348, "e64"),
    (3355, "e65"),
    (3362, "e66"),
    (3369, "e67"),
    (3376, "e68"),
    (3383, "e69"),
    (3390, "e70"),
    (3397, "e71"),
    (3404, "e72"),
    (3411, "e73"),
    (3419, "e74"),
    (3426, "e75"),
    (3433, "e76"),
    (3441, "e77"),
    (3448, "e78"),
    (3455, "e79"),
    (3462, "e80"),
    (3469, "e81"),
    (3476, "e82"),
    (3483, "e83"),
    (3491, "e84"),
];

/// The keys held in `scripts/reference/pause-early.scenario`'s run: P held through the race's
/// load, so the loop's first pass pauses the game before the intro, then Enter; in the race F2
/// twice, F3 twice, then P and Enter (the race's frames from the original's memory).
const PAUSE_EARLY_HELD: [Held; 8] = [
    (2990, Key::P, 40),
    (3213, Key::Enter, 7),
    (3820, Key::F2, 7),
    (4034, Key::F2, 7),
    (4248, Key::F3, 7),
    (4463, Key::F3, 7),
    (4676, Key::P, 7),
    (4962, Key::Enter, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the preview's fade,
/// the box flying in and apart over the black palette, the intro, the race, the game paused.
const PAUSE_EARLY_SHOTS: [(u64, &str); 131] = [
    (2974, "e11"),
    (2981, "e12"),
    (2988, "e13"),
    (2996, "e14"),
    (3003, "e15"),
    (3009, "e16"),
    (3016, "e17"),
    (3023, "e18"),
    (3030, "e19"),
    (3037, "e20"),
    (3044, "e21"),
    (3051, "e22"),
    (3058, "e23"),
    (3065, "e24"),
    (3072, "e25"),
    (3079, "e26"),
    (3086, "e27"),
    (3093, "e28"),
    (3100, "e29"),
    (3107, "e30"),
    (3114, "e31"),
    (3121, "e32"),
    (3128, "e33"),
    (3135, "e34"),
    (3142, "e35"),
    (3150, "e36"),
    (3157, "e37"),
    (3164, "e38"),
    (3171, "e39"),
    (3178, "e40"),
    (3185, "e41"),
    (3192, "e42"),
    (3199, "e43"),
    (3206, "e44"),
    (3213, "e45"),
    (3220, "e46"),
    (3227, "e47"),
    (3234, "e48"),
    (3241, "e49"),
    (3248, "e50"),
    (3255, "e51"),
    (3262, "e52"),
    (3269, "e53"),
    (3276, "e54"),
    (3283, "e55"),
    (3290, "e56"),
    (3297, "e57"),
    (3305, "e58"),
    (3312, "e59"),
    (3319, "e60"),
    (3326, "e61"),
    (3333, "e62"),
    (3340, "e63"),
    (3347, "e64"),
    (3354, "e65"),
    (3362, "e66"),
    (3369, "e67"),
    (3376, "e68"),
    (3383, "e69"),
    (3390, "e70"),
    (3398, "e71"),
    (3405, "e72"),
    (3412, "e73"),
    (3419, "e74"),
    (3426, "e75"),
    (3433, "e76"),
    (3440, "e77"),
    (3447, "e78"),
    (3454, "e79"),
    (3462, "e80"),
    (3469, "e81"),
    (3476, "e82"),
    (3483, "e83"),
    (3490, "e84"),
    (3497, "e85"),
    (3504, "e86"),
    (3512, "e87"),
    (3519, "e88"),
    (3526, "e89"),
    (3533, "e90"),
    (3540, "e91"),
    (3547, "e92"),
    (3554, "e93"),
    (3561, "e94"),
    (3569, "e95"),
    (3604, "s00"),
    (3640, "s01"),
    (3676, "s02"),
    (3712, "s03"),
    (3747, "s04"),
    (3783, "s05"),
    (3819, "s06"),
    (3855, "s07"),
    (3890, "s08"),
    (3926, "s09"),
    (3962, "s10"),
    (3998, "s11"),
    (4033, "s12"),
    (4069, "s13"),
    (4105, "s14"),
    (4140, "s15"),
    (4176, "s16"),
    (4212, "s17"),
    (4248, "s18"),
    (4283, "s19"),
    (4319, "s20"),
    (4355, "s21"),
    (4390, "s22"),
    (4426, "s23"),
    (4462, "s24"),
    (4497, "s25"),
    (4533, "s26"),
    (4569, "s27"),
    (4605, "s28"),
    (4641, "s29"),
    (4676, "s30"),
    (4712, "s31"),
    (4747, "s32"),
    (4783, "s33"),
    (4819, "s34"),
    (4854, "s35"),
    (4890, "s36"),
    (4926, "s37"),
    (4961, "s38"),
    (4997, "s39"),
    (5033, "s40"),
    (5068, "s41"),
    (5104, "s42"),
    (5140, "s43"),
    (5175, "s44"),
    (5211, "s45"),
];

/// The keys held in `scripts/reference/help-early.scenario`'s run: F1 held through the race's
/// load, so the loop's first pass opens the help before the intro, then Enter on each page.
const HELP_EARLY_HELD: [Held; 3] = [
    (2990, Key::F1, 40),
    (3321, Key::Enter, 7),
    (3607, Key::Enter, 7),
];

/// The ticks after which our frame equalled each screenshot of that run: the race's load, the
/// help's pages fading in and out over the black palette, the intro and the countdown.
const HELP_EARLY_SHOTS: [(u64, &str); 154] = [
    (3010, "e16"),
    (3017, "e17"),
    (3024, "e18"),
    (3031, "e19"),
    (3038, "e20"),
    (3046, "e21"),
    (3053, "e22"),
    (3060, "e23"),
    (3067, "e24"),
    (3074, "e25"),
    (3080, "e26"),
    (3087, "e27"),
    (3096, "e28"),
    (3102, "e29"),
    (3109, "e30"),
    (3116, "e31"),
    (3123, "e32"),
    (3130, "e33"),
    (3138, "e34"),
    (3145, "e35"),
    (3152, "e36"),
    (3159, "e37"),
    (3167, "e38"),
    (3174, "e39"),
    (3181, "e40"),
    (3188, "e41"),
    (3195, "e42"),
    (3202, "e43"),
    (3209, "e44"),
    (3216, "e45"),
    (3223, "e46"),
    (3231, "e47"),
    (3238, "e48"),
    (3245, "e49"),
    (3252, "e50"),
    (3259, "e51"),
    (3266, "e52"),
    (3273, "e53"),
    (3280, "e54"),
    (3288, "e55"),
    (3295, "e56"),
    (3302, "e57"),
    (3309, "e58"),
    (3316, "e59"),
    (3323, "e60"),
    (3330, "e61"),
    (3336, "e62"),
    (3343, "e63"),
    (3350, "e64"),
    (3357, "e65"),
    (3364, "e66"),
    (3371, "e67"),
    (3378, "e68"),
    (3385, "e69"),
    (3393, "e70"),
    (3400, "e71"),
    (3407, "e72"),
    (3414, "e73"),
    (3421, "e74"),
    (3428, "e75"),
    (3436, "e76"),
    (3443, "e77"),
    (3451, "e78"),
    (3458, "e79"),
    (3465, "e80"),
    (3473, "e81"),
    (3480, "e82"),
    (3487, "e83"),
    (3494, "e84"),
    (3501, "e85"),
    (3508, "e86"),
    (3515, "e87"),
    (3522, "e88"),
    (3530, "e89"),
    (3537, "e90"),
    (3544, "e91"),
    (3551, "e92"),
    (3558, "e93"),
    (3565, "e94"),
    (3572, "e95"),
    (3579, "e96"),
    (3586, "e97"),
    (3594, "e98"),
    (3601, "e99"),
    (3608, "e100"),
    (3616, "e101"),
    (3621, "e102"),
    (3628, "e103"),
    (3635, "e104"),
    (3643, "e105"),
    (3650, "e106"),
    (3657, "e107"),
    (3664, "e108"),
    (3672, "e109"),
    (3679, "e110"),
    (3686, "e111"),
    (3693, "e112"),
    (3700, "e113"),
    (3707, "e114"),
    (3715, "e115"),
    (3722, "e116"),
    (3729, "e117"),
    (3735, "e118"),
    (3743, "e119"),
    (3750, "e120"),
    (3757, "e121"),
    (3764, "e122"),
    (3771, "e123"),
    (3778, "e124"),
    (3786, "e125"),
    (3793, "e126"),
    (3800, "e127"),
    (3807, "e128"),
    (3814, "e129"),
    (3821, "e130"),
    (3828, "e131"),
    (3835, "e132"),
    (3842, "e133"),
    (3850, "e134"),
    (3857, "e135"),
    (3864, "e136"),
    (3871, "e137"),
    (3878, "e138"),
    (3886, "e139"),
    (3893, "e140"),
    (3900, "e141"),
    (3907, "e142"),
    (3914, "e143"),
    (3921, "e144"),
    (3928, "e145"),
    (3936, "e146"),
    (3943, "e147"),
    (3950, "e148"),
    (3957, "e149"),
    (3964, "e150"),
    (3971, "e151"),
    (3979, "e152"),
    (3986, "e153"),
    (3993, "e154"),
    (4000, "e155"),
    (4007, "e156"),
    (4014, "e157"),
    (4021, "e158"),
    (4029, "e159"),
    (4036, "e160"),
    (4043, "e161"),
    (4050, "e162"),
    (4057, "e163"),
    (4064, "e164"),
    (4071, "e165"),
    (4079, "e166"),
    (4086, "e167"),
    (4093, "e168"),
    (4100, "e169"),
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
/// with `held` keys besides the ones pressed and let go at once, the opponents kept still
/// as the original's runs with `--no-ai` keep them.
fn manifest_seeded(
    start: (u32, Option<u32>),
    keys: (&[(u64, Key)], &[Held]),
    shots: &[(u64, &str)],
    ticks: u64,
    slots: Vec<Option<Vec<u8>>>,
) -> String {
    manifest_run(start, keys, shots, ticks, slots, true)
}

/// [`manifest_seeded`] with the opponents driving unless `still`.
fn manifest_run(
    (seed, clock): (u32, Option<u32>),
    (keys, held): (&[(u64, Key)], &[Held]),
    shots: &[(u64, &str)],
    ticks: u64,
    slots: Vec<Option<Vec<u8>>>,
    still: bool,
) -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let config = assets.menu.default_config.clone();
    let mut game = Game::with_seed(assets, config, seed);
    game.set_saved_games(slots);
    if let Some(ms) = clock {
        game.fix_sabotage_clock(ms);
    }
    if still {
        game.keep_opponents_still();
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

/// Two races in a row with a rocket: the rocket run's way into the race, the turbo held 75
/// ticks (the flames turning to their second picture last), Escape and Y; then, as the
/// original's run does, Enter every two seconds through the results, the shop, the
/// Underground Market and the sign-up into the next race.
const TWO_RACES_HELD: [Held; 4] = [
    (3388, Key::Up, 122),
    (3403, Key::LeftShift, 75),
    (3540, Key::Escape, 7),
    (3670, Key::Y, 7),
];
const TWO_RACES_ENTER_EVERY: u64 = 140;

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn a_later_race_starts_its_rocket_flames_where_the_last_race_left_them() {
    // The original never sets the flames' picture (0x456AFC) back: only a flame's turn writes
    // it (0x40F651). A player who ends a race on the second picture sees the next race's first
    // flame in it, not in the first.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(armed_save(&assets.menu.texts, 37, [0, 0, 1]));
    let config = assets.menu.default_config.clone();
    let mut game = Game::with_seed(assets, config, SEED);
    game.set_saved_games(slots);
    game.keep_opponents_still();
    // Each race's flame pictures, tick by tick.
    let mut races: Vec<Vec<String>> = Vec::new();
    let mut racing = false;
    for done in 0..9_000 {
        if races.len() == 1 && !racing && done % TWO_RACES_ENTER_EVERY == 0 {
            for pressed in [true, false] {
                game.input(InputEvent::Key {
                    key: Key::Enter,
                    pressed,
                });
            }
        }
        for &(_, key) in RACE_START_KEYS.iter().filter(|(at, _)| *at == done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        for &(at, key, ticks) in &TWO_RACES_HELD {
            if at == done || at + ticks == done {
                game.input(InputEvent::Key {
                    key,
                    pressed: at == done,
                });
            }
        }
        game.tick();
        let trace = game.race_trace();
        if let Some(trace) = &trace {
            if !racing {
                races.push(Vec::new());
            }
            let phase = trace.split_whitespace().nth(1).unwrap_or_default();
            races.last_mut().unwrap().push(phase.to_owned());
        }
        racing = trace.is_some();
    }
    assert_eq!(races.len(), 2, "two races");
    assert_eq!(races[0].last().map(String::as_str), Some("fp1"));
    assert_eq!(races[1].first().map(String::as_str), Some("fp1"));
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

/// The keys held in `scripts/reference/wreck.scenario`'s run with the music off
/// (docs/verification/m5.md): the mine key, Down backing over the mine, and Enter once the
/// race is over.
const QUIET_WRECK_HELD: [Held; 3] = [
    (3640, Key::LeftAlt, 7),
    (3698, Key::Down, 49),
    (4184, Key::Enter, 7),
];

/// The sound of a run seeded as the reference runs are, the opponents still, from the saved
/// game `save` with the race-start keys and `held`, under `config`, for `ticks`.
fn run_sound(save: Vec<u8>, held: &[Held], ticks: u64, config: DrCfg) -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut game = Game::with_seed(assets, config, SEED);
    let mut slots = vec![None; 8];
    slots[0] = Some(save);
    game.set_saved_games(slots);
    game.keep_opponents_still();
    let mut audio = Vec::new();
    for done in 0..ticks {
        for &(_, key) in RACE_START_KEYS.iter().filter(|(at, _)| *at == done) {
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
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    hex(hasher)
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_race_effects_sound_matches_the_committed_manifest() {
    // Written after the recordings of the original with the music off measured as ours
    // (docs/verification/m5.md): the mines dropped and the blast, the horn, the wreck's fire
    // and the race's end call, each where and as loud as the original plays it.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let texts = &assets.menu.texts;
    let mut quiet = assets.menu.default_config.clone();
    quiet.set_music_volume(0);
    let mines = run_sound(
        armed_save(texts, 37, [3, 0, 0]),
        &MINES_HELD,
        3_900,
        quiet.clone(),
    );
    let wreck = run_sound(
        armed_save(texts, 99, [1, 0, 0]),
        &QUIET_WRECK_HELD,
        4_300,
        quiet,
    );
    let lines = format!(
        "{mines}  the mines and the horn, the music off, 3900 ticks\n\
         {wreck}  the wreck and the race's end, the music off, 4300 ticks\n"
    );
    check_manifest("race-effects-sound.sha256", &lines, "the race's effects");
}

/// The keys held in `scripts/reference/damage-calls.scenario`'s run, from the original's
/// memory: each of the two mines dropped and the car backed over it.
const DAMAGE_CALLS_HELD: [Held; 4] = [
    (3631, Key::LeftAlt, 7),
    (3688, Key::Down, 50),
    (3793, Key::LeftAlt, 7),
    (3850, Key::Down, 50),
];

/// `save` with the player renamed `name` (the first 12 bytes of the player's record).
fn renamed(save: &[u8], name: &[u8]) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(save);
    let at = usize::from(game.driver_id) * 108;
    let record = &mut game.drivers[at..at + 12];
    record.fill(0);
    let length = name.len().min(11);
    record[..length].copy_from_slice(&name[..length]);
    game.encode(77)
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_damage_calls_sound_matches_the_committed_manifest() {
    // Written after recordings of the original with the music off measured as ours
    // (docs/verification/m5.md): the HUD's warning as the damage bar falls under a fifth and
    // again under a tenth, each once a race, and the tough driver's own call when he is
    // wrecked. A warning missing, repeated or late is what this pins.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let texts = &assets.menu.texts;
    let mut quiet = assets.menu.default_config.clone();
    quiet.set_music_volume(0);
    let calls = run_sound(
        armed_save(texts, 75, [2, 0, 0]),
        &DAMAGE_CALLS_HELD,
        4_100,
        quiet.clone(),
    );
    // The tough driver's name as dr.exe keeps it, typed as a player would type it: only its
    // words' first letters capitals, so the race's upper-casing counts too.
    let tough = &assets.race.handling.tough;
    let name: Vec<u8> = tough
        .iter()
        .take_while(|&&c| c != 0)
        .enumerate()
        .map(|(at, &c)| {
            if at == 0 || tough[at - 1] == b' ' {
                c
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    let wreck = run_sound(
        renamed(&armed_save(texts, 99, [1, 0, 0]), &name),
        &WRECK_HELD,
        4_300,
        quiet,
    );
    let lines = format!(
        "{calls}  the damage bar under a fifth and a tenth, the music off, 4100 ticks\n\
         {wreck}  the tough driver wrecked, the music off, 4300 ticks\n"
    );
    check_manifest("damage-calls-sound.sha256", &lines, "the damage calls");
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
fn the_opponents_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m5.md): the
    // opponents' steering, speed, laps and places, and every draw of `rand()` they make.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_run(
        (SEED, None),
        (&RACE_START_KEYS, &[]),
        &OPPONENTS_SHOTS,
        7_740,
        slots,
        false,
    );
    check_manifest("opponents-run.sha256", &lines, "the opponents run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_effect_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m5.md): a
    // view wavering differently, or for longer, under the effect power-up shows here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &EFFECT_HELD),
        &EFFECT_SHOTS,
        4_720,
        slots,
    );
    check_manifest("effect-run.sha256", &lines, "the effect run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_race_keys_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m5.md): the
    // status bar sliding at the wrong speed, a view centred wrongly without it, a missing small
    // board, shadows or pictures that do not switch, or the paused box out of place show here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &RACE_KEYS_HELD),
        &RACE_KEYS_SHOTS,
        4_290,
        slots,
    );
    check_manifest("race-keys-run.sha256", &lines, "the race keys run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_lap_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and every
    // frame's state of the cars equalled the original's memory (docs/verification/m5.md): a
    // whole lap of the player's car, the HUD's clock, the lap's time shown for its 210 ticks
    // (counted down twice a frame by the small board) and the small board's time.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(unarmed_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &LAP_HELD),
        &LAP_SHOTS,
        5_870,
        slots,
    );
    check_manifest("lap-run.sha256", &lines, "the lap run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_results_run_matches_the_committed_manifest() {
    // Written after the screenshots of the run equalled our frames (docs/verification/m5.md):
    // the race to a wreck with the opponents driving, the results fading in with the medium
    // race's page at once (the race's last Enter), the hard race's, every first three's
    // points, the standings sorted afresh with the player's statistics, the way back out
    // through the Underground Market's fade, and the shop after them with the last place's
    // popup (the wrecked player's, after the fade, the cursor from its first frame), drawn
    // again after it, and the Underground Market refusing the wreck.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(armed_save(&assets.menu.texts, 99, [1, 0, 0]));
    let lines = manifest_run(
        (SEED, None),
        (&RESULTS_KEYS, &RESULTS_HELD),
        &RESULTS_SHOTS,
        7_160,
        slots,
        false,
    );
    check_manifest("results-run.sha256", &lines, "the results run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_cheats_run_matches_the_committed_manifest() {
    // Written after the screenshots of the run equalled our frames (docs/verification/m5.md):
    // DRAW's $1000, DROOL's $500000, DRIVE's and DROP's ranks after the standings are sorted
    // afresh, in the shop's side panel.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&CHEATS_KEYS, &[]),
        &CHEATS_SHOTS,
        2_800,
        slots,
    );
    check_manifest("cheats-run.sha256", &lines, "the cheats run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_no_sign_up_run_matches_the_committed_manifest() {
    // Written after the screenshots of the run equalled our frames (docs/verification/m5.md):
    // signing up for no race fills every race with the others, draws the cars' places, and
    // shows the three races' results, every one placed by car, then the statistics.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&NO_SIGN_UP_KEYS, &[]),
        &NO_SIGN_UP_SHOTS,
        5_700,
        slots,
    );
    check_manifest("no-sign-up-run.sha256", &lines, "the no sign-up run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_arena_run_matches_the_committed_manifest() {
    // Written after 363 of the run's 374 screenshots equalled our frames and every frame of
    // the race's state, rand()'s included, equalled the original's memory
    // (docs/verification/m6.md): a leader's Enter on the Adversary's screen leads to the
    // Arena's preview of two cars and the race against car 6; the Adversary's handling, guns
    // and driving, the flag at the finish, and the races drawn after it show here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(leader_save(&assets.menu.texts));
    let lines = manifest_run(
        (SEED, None),
        (&ARENA_KEYS, &ARENA_HELD),
        &ARENA_SHOTS,
        12_200,
        slots,
        false,
    );
    check_manifest("arena-run.sha256", &lines, "the Arena run");
}

/// The won Arena run's keys (`arena-won.keys`: tick, key, ticks held): the player's, from the
/// original's memory (`scripts/compare-watch.py --keys`, tick = race frame + 3016), Return
/// held on the race-over box and on the best ten.
fn arena_won_held() -> Vec<Held> {
    let mut held: Vec<Held> = include_str!("arena-won.keys")
        .lines()
        .map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            let key = match fields[1] {
                "up" => Key::Up,
                "left" => Key::Left,
                "right" => Key::Right,
                other => panic!("arena-won.keys: unknown key {other}"),
            };
            (fields[0].parse().unwrap(), key, fields[2].parse().unwrap())
        })
        .collect();
    held.extend([(15_181, Key::Enter, 7), (19_253, Key::Enter, 7)]);
    held
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_arena_won_run_matches_the_committed_manifest() {
    // Written after all 508 screenshots of the run equalled our frames and every frame of the
    // race's state, rand()'s included, equalled the original's memory (docs/verification/
    // m6.md): the leader wins the Arena, and the end follows, the title, the best ten with the
    // winner put in and dr.cfg written. A wrong end, Hall of Fame entry or way back to the
    // menus shows here; the menu keys up to Enter on the Adversary's screen are the lost run's.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(leader_save(&assets.menu.texts));
    let shots: Vec<(u64, String)> = include_str!("arena-won.shots")
        .lines()
        .map(|line| {
            let (tick, name) = line.split_once(' ').unwrap();
            (tick.parse().unwrap(), name.to_owned())
        })
        .collect();
    let shots: Vec<(u64, &str)> = shots.iter().map(|(t, n)| (*t, n.as_str())).collect();
    let lines = manifest_seeded(
        (SEED, None),
        (&ARENA_KEYS[..9], &arena_won_held()),
        &shots,
        21_000,
        slots,
    );
    check_manifest("arena-won-run.sha256", &lines, "the won Arena run");
}

/// The keys of `scripts/reference/leader-turn.scenario`'s run of `docs/verification/m6.md`
/// after the race start's: Enter on the hard race's page, on the statistics, on their way out
/// and on the shop's way on.
const LEADER_TURN_KEYS: [(u64, Key); 4] = [
    (13_180, Key::Enter),
    (13_537, Key::Enter),
    (13_890, Key::Enter),
    (16_531, Key::Enter),
];

/// A leader turn run's held keys (`file`, its lines tick, key, ticks held): the player's car's
/// four laps, from the original's memory (`scripts/compare-watch.py --keys`), then Return held
/// on the race-over box at `box_tick`.
fn leader_turn_held(file: &str, text: &str, box_tick: u64) -> Vec<Held> {
    let mut held: Vec<Held> = text
        .lines()
        .map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            let key = match fields[1] {
                "up" => Key::Up,
                "left" => Key::Left,
                "right" => Key::Right,
                other => panic!("{file}: unknown key {other}"),
            };
            (fields[0].parse().unwrap(), key, fields[2].parse().unwrap())
        })
        .collect();
    held.push((box_tick, Key::Enter, 7));
    held
}

/// A leader turn run's shots (`text`, its lines tick and name).
fn leader_turn_shots(text: &str) -> Vec<(u64, &str)> {
    text.lines()
        .map(|line| {
            let (tick, name) = line.split_once(' ').unwrap();
            (tick.parse().unwrap(), name)
        })
        .collect()
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_leader_turn_run_matches_the_committed_manifest() {
    // Written after 256 of the run's 261 screenshots equalled our frames and every frame of the
    // race's state, rand()'s included, equalled the original's memory (docs/verification/
    // m6.md): the race won that first makes the player the leader ends its results with every
    // colour faded out, the Adversary's animation and the menus' music, then the shop, whose
    // way on leads to the Adversary. A way out that keeps the title lit, skips the animation or
    // comes back to the wrong screen, or a leader not found, shows here.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(leader_turn_save(&assets.menu.texts));
    let keys: Vec<(u64, Key)> = RACE_START_KEYS
        .iter()
        .chain(&LEADER_TURN_KEYS)
        .copied()
        .collect();
    let held = leader_turn_held("leader-turn.keys", include_str!("leader-turn.keys"), 12_823);
    let lines = manifest_seeded(
        (SEED, None),
        (&keys, &held),
        &leader_turn_shots(include_str!("leader-turn.shots")),
        16_830,
        slots,
    );
    check_manifest("leader-turn-run.sha256", &lines, "the leader turn run");
}

/// The keys of the leader turn's run with weapons (`leader-turn.scenario` with
/// `leader_turn_armed_save`, docs/verification/m6.md): the race start's, as that run's original
/// took them (the market's waits draw `rand()`, so its keys decide the opponents' weapons),
/// then Enter on the hard race's page, on the statistics, on their way out and on the shop's
/// way on.
const LEADER_TURN_MARKET_KEYS: [(u64, Key); 14] = [
    (130, Key::Space),
    (1728, Key::Enter),
    (1806, Key::Down),
    (1870, Key::Enter),
    (1952, Key::Enter),
    (2037, Key::Space),
    (2166, Key::Enter),
    (2419, Key::Enter),
    (2613, Key::Enter),
    (2788, Key::Space),
    (13_179, Key::Enter),
    (13_541, Key::Enter),
    (13_893, Key::Enter),
    (16_535, Key::Enter),
];

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn after_the_leader_s_animation_the_market_s_way_out_fades_a_black_screen() {
    // The original clears the screen it shows after the Adversary's animation (`sub_43BE60`
    // from 0x42B657), and on the way out that keeps the screen (weapons on, the shop's way on
    // seen) the Underground Market's fade (0x4370C7) draws nothing over it: the player sees
    // black until the shop fades in, not the results' statistics again (docs/verification/
    // m6.md, the run with weapons).
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(leader_turn_armed_save(&assets.menu.texts));
    let config = assets.menu.default_config.clone();
    let mut game = Game::with_seed(assets, config, SEED);
    game.set_saved_games(slots);
    game.keep_opponents_still();
    let held = leader_turn_held(
        "leader-turn-market.keys",
        include_str!("leader-turn-market.keys"),
        12_827,
    );
    let mut film_seen = false;
    for done in 0..16_000 {
        for &(_, key) in LEADER_TURN_MARKET_KEYS.iter().filter(|(at, _)| *at == done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        for &(at, key, ticks) in &held {
            if at == done || at + ticks == done {
                game.input(InputEvent::Key {
                    key,
                    pressed: at == done,
                });
            }
        }
        game.tick();
        let frame = game.frame();
        // The animation follows the results' way out (from tick 13 893); the intro at the
        // start is a film 320 wide too.
        if done < 13_900 {
            continue;
        }
        if frame.width == 320 {
            film_seen = true;
        } else if film_seen {
            // The market's fade has begun: its first waits show the cleared screen at full
            // brightness, so anything drawn would show.
            game.tick();
            let frame = game.frame();
            assert!(
                frame.pixels.iter().all(|&pixel| pixel == 0),
                "the market's fade after the animation shows a cleared screen (tick {done})"
            );
            return;
        }
    }
    panic!("the leader's animation was not seen");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_leader_turn_market_run_matches_the_committed_manifest() {
    // Written after 260 of the run's 261 screenshots equalled our frames and every frame of
    // the race's state equalled the original's memory (docs/verification/m6.md): with weapons
    // on, the results that first make the player the leader keep the screen while every colour
    // fades, the Adversary's animation plays, the Underground Market fades a black screen out,
    // the shop fades in and its way on leads to the market.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(leader_turn_armed_save(&assets.menu.texts));
    let held = leader_turn_held(
        "leader-turn-market.keys",
        include_str!("leader-turn-market.keys"),
        12_827,
    );
    let lines = manifest_seeded(
        (SEED, None),
        (&LEADER_TURN_MARKET_KEYS, &held),
        &leader_turn_shots(include_str!("leader-turn-market.shots")),
        16_840,
        slots,
    );
    check_manifest(
        "leader-turn-market-run.sha256",
        &lines,
        "the leader turn run with weapons",
    );
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_adversary_run_matches_the_committed_manifest() {
    // Written after the screenshots of the run equalled our frames (docs/verification/m6.md):
    // a leader goes from the market's way on to the Adversary's screen, and Escape on it fills
    // the races and shows their results.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(leader_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&ADVERSARY_KEYS, &[]),
        &ADVERSARY_SHOTS,
        3_860,
        slots,
    );
    check_manifest("adversary-run.sha256", &lines, "the Adversary run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_statistics_run_matches_the_committed_manifest() {
    // Written after the screenshots of the run equalled our frames (docs/verification/m5.md):
    // the Start Racing menu out but for the title's colours, the player's statistics without a
    // race's part beside the standings, the blinking line, and the menus back.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&STATISTICS_KEYS, &[]),
        &STATISTICS_SHOTS,
        3_100,
        slots,
    );
    check_manifest("statistics-run.sha256", &lines, "the statistics run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_shop_welcome_run_matches_the_committed_manifest() {
    // Written after the screenshots of the run equalled our frames (docs/verification/m5.md,
    // m6.md): a new game's first race with its welcome box, abandoned, its results, and the
    // shop after them with the welcome to it drawn before its fade and without the continue
    // item's border, then the last place's popup, then the shop drawn again.
    let keys: Vec<(u64, Key)> = NEW_GAME_KEYS
        .iter()
        .chain(&SHOP_WELCOME_KEYS)
        .copied()
        .collect();
    let lines = manifest_seeded(
        (SEED, None),
        (&keys, &SHOP_WELCOME_HELD),
        &SHOP_WELCOME_SHOTS,
        6_500,
        Vec::new(),
    );
    check_manifest("shop-welcome-run.sha256", &lines, "the shop welcome run");
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
fn the_abort_early_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m5.md): a race abandoned before its intro must still show the intro
    // before the view tilts away.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &ABORT_EARLY_HELD),
        &ABORT_EARLY_SHOTS,
        3_560,
        slots,
    );
    check_manifest("abort-early-run.sha256", &lines, "the abort-early run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_pause_early_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, every frame
    // of the race's state equalled the original's memory and the recording measured as ours
    // (docs/verification/m5.md): P held as the race loads must pause the game before the
    // intro, with the music's calmer order at half volume; F2, F3 and P in the race silence
    // the music, the effects and the race as the original does.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &PAUSE_EARLY_HELD),
        &PAUSE_EARLY_SHOTS,
        5_300,
        slots,
    );
    check_manifest("pause-early-run.sha256", &lines, "the pause-early run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_help_early_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run from the race's load on equalled our frame at
    // its tick (docs/verification/m5.md): F1 held as the race loads must open the help before
    // the intro, its pages fading in and out over the black palette.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &HELP_EARLY_HELD),
        &HELP_EARLY_SHOTS,
        4_300,
        slots,
    );
    check_manifest("help-early-run.sha256", &lines, "the help-early run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn p_held_while_the_race_loads_pauses_the_game_before_the_intro() {
    // The race loop's first pass checks the race's keys before its drawing as every pass
    // does (0x416D13): P held through the race's load opens the "game paused" box before the
    // intro, over the palette still black. A player pressing P as the race comes must find
    // the game paused, not the race started without them.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let config = assets.menu.default_config.clone();
    let mut game = Game::with_seed(assets, config, SEED);
    game.set_saved_games(slots);
    game.keep_opponents_still();
    let held: [Held; 2] = [(2990, Key::P, 40), (3400, Key::Enter, 7)];
    let mut black_until = None;
    for done in 0..3_600 {
        for &(_, key) in RACE_START_KEYS.iter().filter(|(at, _)| *at == done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        for &(at, key, ticks) in &held {
            if at == done || at + ticks == done {
                game.input(InputEvent::Key {
                    key,
                    pressed: at == done,
                });
            }
        }
        game.tick();
        let lit = game.frame().palette.iter().any(|&colour| colour != [0; 3]);
        if game.race_trace().is_some() && lit && black_until.is_none() {
            black_until = Some(done);
        }
    }
    let lit = black_until.expect("the race's intro came");
    assert!(
        lit > 3400,
        "the intro waited for the box's Enter (lit at {lit})"
    );
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_tab_abort_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m5.md): a race abandoned with the status bar hidden must spin away
    // round its top left corner, not tilt.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &TAB_ABORT_HELD),
        &TAB_ABORT_SHOTS,
        3_800,
        slots,
    );
    check_manifest("tab-abort-run.sha256", &lines, "the tab-abort run");
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

/// The test game with the player leading every other driver on points (150 against the best
/// other's 100; `captures/leader.sg`).
fn leader_save(texts: &deadrally_gamedata::text::Texts) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(&test_save(texts));
    let at = 19 * 108 + 68;
    game.drivers[at..at + 4].copy_from_slice(&150i32.to_le_bytes());
    game.encode(77)
}

/// The test game without weapons whose player (85 points) is one behind the leader (Jane
/// Honda's 86) and whose first driver (Sam Speed, who wins the hard race) has 70: only a win of
/// the easy race (3 points) makes the player lead (`captures/leader-turn.sg`).
fn leader_turn_save(texts: &deadrally_gamedata::text::Texts) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(&unarmed_save(texts));
    for (driver, points) in [(19, 85i32), (0, 70)] {
        let at = driver * 108 + 68;
        game.drivers[at..at + 4].copy_from_slice(&points.to_le_bytes());
    }
    game.encode(77)
}

/// The test game, its weapons on, whose player (99 points) is one behind the leader (Sam
/// Speed's 100, in no race): winning the easy race (3 points) makes the player lead, the hard
/// race's winner reaching 96 (`captures/leader-turn-armed.sg`).
fn leader_turn_armed_save(texts: &deadrally_gamedata::text::Texts) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(&test_save(texts));
    let at = 19 * 108 + 68;
    game.drivers[at..at + 4].copy_from_slice(&99i32.to_le_bytes());
    game.encode(77)
}

/// The test game with its weapons switched off (`captures/test.sg` with the weapons byte 0).
fn unarmed_save(texts: &deadrally_gamedata::text::Texts) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(&test_save(texts));
    game.use_weapons = 0;
    game.encode(77)
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
