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

/// The ticks after which our frame equalled each screenshot of that run: the box landed, and
/// flying apart before the race ends.
const ABORT_SHOTS: [(u64, &str); 9] = [
    (3555, "box"),
    (3584, "a01"),
    (3589, "a02"),
    (3597, "a03"),
    (3604, "a04"),
    (3611, "a05"),
    (3618, "a06"),
    (3625, "a07"),
    (3632, "a08"),
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
fn the_abort_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick
    // (docs/verification/m4b.md): the box landed, Y, the tiles flying apart.
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut slots = vec![None; 8];
    slots[0] = Some(test_save(&assets.menu.texts));
    let lines = manifest_seeded(
        (SEED, None),
        (&RACE_START_KEYS, &ABORT_HELD),
        &ABORT_SHOTS,
        3_640,
        slots,
    );
    check_manifest("abort-run.sha256", &lines, "the abort run");
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
            37, 1, 1, 0, 1, 0, 0, 0, 60, 23456, -1, -1, price, 5, 41, 12, 0, 0, 0, 0, 0, 0, 0, 0,
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
