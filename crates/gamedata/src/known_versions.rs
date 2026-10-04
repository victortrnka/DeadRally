/// The data files the game needs (brief §9), in canonical upper case. Names on disk match
/// case-insensitively.
pub const REQUIRED_FILES: [&str; 19] = [
    "ENGINE.BPA",
    "IBFILES.BPA",
    "MENU.BPA",
    "MUSICS.BPA",
    "TR0.BPA",
    "TR1.BPA",
    "TR2.BPA",
    "TR3.BPA",
    "TR4.BPA",
    "TR5.BPA",
    "TR6.BPA",
    "TR7.BPA",
    "TR8.BPA",
    "TR9.BPA",
    "SANIM.HAF",
    "ENDANI.HAF",
    "ENDANI0.HAF",
    "END.BMP",
    "RMD.BMP",
];

/// One file of a known release. Hashes are facts about the data, not the data itself, so they
/// may be committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownFile {
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

/// A release of the game whose data we have verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownVersion {
    pub name: &'static str,
    pub files: &'static [KnownFile],
}

/// Every release we recognise. Remedy's 2009 freeware download is added once someone hashes
/// a copy; until then it validates as an unknown version.
pub const KNOWN_VERSIONS: &[KnownVersion] = &[STEAM_358270];

/// Downloaded with steamcmd on 2026-10-03 (Windows depot).
const STEAM_358270: KnownVersion = KnownVersion {
    name: "Steam: Death Rally (Classic), appid 358270",
    files: &[
        KnownFile {
            name: "ENGINE.BPA",
            size: 387_193,
            sha256: "8ff2aff5b4c5d10a1ada9c7529b95bded949d4914f917e6dc6e2fe5448cf14c2",
        },
        KnownFile {
            name: "IBFILES.BPA",
            size: 85_989,
            sha256: "93f08b317df159aeb229d6ba86bb1c6f40ba9d28fdc10dd81e67639348dc4c74",
        },
        KnownFile {
            name: "MENU.BPA",
            size: 3_168_933,
            sha256: "b0993e984066a7e5e7edcfca69476624872a7f322648b0b4961f3c05339f15b1",
        },
        KnownFile {
            name: "MUSICS.BPA",
            size: 5_726_559,
            sha256: "9f1e094f2a8763683027614fd97cb8afce5836c158f0d1b25be2842bf334754b",
        },
        KnownFile {
            name: "TR0.BPA",
            size: 337_303,
            sha256: "368fa56187b8e1ca480e7ec1a6f5aad655d259e8b4bc398bc08131eaed4dd45f",
        },
        KnownFile {
            name: "TR1.BPA",
            size: 321_394,
            sha256: "efa9cd8bec6c240675ef6bfe333638442436c4229cbb9c74772257b8683d4305",
        },
        KnownFile {
            name: "TR2.BPA",
            size: 294_365,
            sha256: "7f242b788f920e4b00f254f30e49bc98536d7cc58513056b456236a0f4821130",
        },
        KnownFile {
            name: "TR3.BPA",
            size: 345_851,
            sha256: "d1d7b5db7d6b3529d7c953557c3a76e7570283f9833a306447d9417a95db7b9f",
        },
        KnownFile {
            name: "TR4.BPA",
            size: 260_466,
            sha256: "238be60f379ea282647ed33157a087b519854cdb821d192878f78f9ef9b0af0d",
        },
        KnownFile {
            name: "TR5.BPA",
            size: 431_992,
            sha256: "ab0767d3acf7c1f45bdfb3124a4206cfe3cadb650602a399d33a05b521e5a9dd",
        },
        KnownFile {
            name: "TR6.BPA",
            size: 359_368,
            sha256: "757587f57fe6711b6353a054e654dedcc5b2553d82476c310912e3ac6339e6c2",
        },
        KnownFile {
            name: "TR7.BPA",
            size: 417_431,
            sha256: "c11dc15b3e8f5e2708d0c6e0cca73ac72ce9d8e88a533e11a3a9c006608805c2",
        },
        KnownFile {
            name: "TR8.BPA",
            size: 253_202,
            sha256: "c7bfb5142cbc7c95dd42b9c40b298d2e3b3af9bb9d445b6c441369e6dfa24036",
        },
        KnownFile {
            name: "TR9.BPA",
            size: 427_555,
            sha256: "3c6336f43d7188fc1f1d4d53307ceea43f64342f9262f9a298e28834b5e36912",
        },
        KnownFile {
            name: "SANIM.HAF",
            size: 21_516_352,
            sha256: "4fbb589fe50f8aa7e47ae41245c64578a9112ea843be2089178a08333087c993",
        },
        KnownFile {
            name: "ENDANI.HAF",
            size: 3_796_331,
            sha256: "13eef7520ae0a5180484a32a4a31efcbb7438f050c7e96f158cc59fb67e2526f",
        },
        KnownFile {
            name: "ENDANI0.HAF",
            size: 7_193_340,
            sha256: "cd20359a766a6ee64137869f65caa91ab6b166dc314ef1ef0561943f7b54a362",
        },
        KnownFile {
            name: "END.BMP",
            size: 308_148,
            sha256: "8723a70d39ca89a092765ce3813b7f34bb6168bdc96e7d635013bcd60bb6d190",
        },
        KnownFile {
            name: "RMD.BMP",
            size: 308_278,
            sha256: "af3cdbcecfeb40aaf9952b19149fadbe40cc46641db90cdd30d12d88598dd972",
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_version_lists_exactly_the_required_files() {
        // A version that forgot a file could never validate as known; one with an extra file
        // would demand data the game does not use.
        for version in KNOWN_VERSIONS {
            let mut names: Vec<_> = version.files.iter().map(|f| f.name).collect();
            names.sort_unstable();
            let mut required = REQUIRED_FILES.to_vec();
            required.sort_unstable();
            assert_eq!(names, required, "{}", version.name);
        }
    }

    #[test]
    fn known_hashes_are_lowercase_sha256_hex() {
        // Validation compares hex strings exactly; an uppercase or truncated entry never matches.
        for file in KNOWN_VERSIONS.iter().flat_map(|v| v.files) {
            assert_eq!(file.sha256.len(), 64, "{}", file.name);
            assert!(
                file.sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "{}",
                file.name
            );
        }
    }
}
