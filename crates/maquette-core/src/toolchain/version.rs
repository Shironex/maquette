//! Version parsers for each tool's `--version` output. Hand rolled so the
//! crate needs no regex engine.

use super::ToolId;

/// Tries stdout first, then stderr (some tools print their banner there).
pub fn parse(tool: ToolId, stdout: &str, stderr: &str) -> Option<String> {
    let parser: fn(&str) -> Option<String> = match tool {
        ToolId::Blender => blender,
        ToolId::Codex => codex,
        ToolId::Ffmpeg => ffmpeg,
        ToolId::Toktx => toktx,
        ToolId::GltfTransform => gltf_transform,
        ToolId::Node => node,
    };
    parser(stdout).or_else(|| parser(stderr))
}

/// `Blender 4.3.2`, possibly after warning lines such as `Read prefs: ...`.
pub fn blender(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("Blender ")?;
        numeric_at(rest.trim_start(), 1).map(str::to_string)
    })
}

/// `codex-cli 0.46.0` or `codex 1.2.3`.
pub fn codex(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        let (head, rest) = line.split_once(char::is_whitespace)?;
        if !head.eq_ignore_ascii_case("codex") && !head.eq_ignore_ascii_case("codex-cli") {
            return None;
        }
        let token = rest.trim_start();
        let token = token.strip_prefix('v').unwrap_or(token);
        numeric_at(token, 1).map(str::to_string)
    })
}

/// `ffmpeg version 7.1`, `n7.0.2-...`, `6.1.1-3ubuntu5`. Git builds such as
/// `2024-11-25-git-04ce01df0b-full_build` report `git 2024-11-25`, and
/// `N-113684-g...` nightlies report `git N-113684`.
pub fn ffmpeg(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (_, rest) = line.split_once("ffmpeg version ")?;
        let token = rest.split_whitespace().next()?;
        let numeric = token
            .strip_prefix('n')
            .filter(|t| t.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap_or(token);
        if let Some(date) = git_date(numeric) {
            return Some(format!("git {date}"));
        }
        if let Some(nightly) = nightly_build(numeric) {
            return Some(format!("git {nightly}"));
        }
        numeric_at(numeric, 1).map(str::to_string)
    })
}

/// `toktx v4.3.2`, `v4.4.0~5` (the `~` build count is dropped) and
/// `v4.3.0-rc1` (a pre-release tag is kept).
pub fn toktx(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (_, rest) = line.split_once("toktx")?;
        let rest = rest.trim_start_matches([' ', ':']).trim_start();
        let rest = rest.strip_prefix("version").unwrap_or(rest);
        let rest = rest.trim_start_matches([' ', ':']);
        let token = rest.strip_prefix('v').unwrap_or(rest);
        let core = numeric_at(token, 1)?;
        let after = &token[core.len()..];
        match after.strip_prefix('-') {
            Some(tail) => {
                let pre: String = tail
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if pre.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    Some(format!("{core}-{pre}"))
                } else {
                    Some(core.to_string())
                }
            }
            None => Some(core.to_string()),
        }
    })
}

/// Usually a bare `4.1.1`, sometimes after a banner. Takes the first
/// `x.y.z` token.
pub fn gltf_transform(output: &str) -> Option<String> {
    first_numeric(output, 2)
}

/// `v22.23.0`.
pub fn node(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix('v')?;
        numeric_at(rest, 2).map(str::to_string)
    })
}

/// Longest `digits(.digits)*` prefix of `s` with at least `min_dots` dots.
fn numeric_at(s: &str, min_dots: usize) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut end = 0;
    let mut dots = 0;
    let mut last_good = None;
    while end < bytes.len() {
        let start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            break;
        }
        if dots >= min_dots {
            last_good = Some(end);
        }
        if end < bytes.len()
            && bytes[end] == b'.'
            && bytes.get(end + 1).is_some_and(u8::is_ascii_digit)
        {
            dots += 1;
            end += 1;
        } else {
            break;
        }
    }
    last_good.map(|e| &s[..e])
}

/// First version token in `s` that starts at a word boundary (an optional
/// leading `v` is allowed).
fn first_numeric(s: &str, min_dots: usize) -> Option<String> {
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if !b.is_ascii_digit() {
            continue;
        }
        let prev = i.checked_sub(1).map(|p| bytes[p]);
        let boundary = match prev {
            None => true,
            Some(b'v') | Some(b'V') => i < 2 || !bytes[i - 2].is_ascii_alphanumeric(),
            Some(p) => !p.is_ascii_alphanumeric() && p != b'.' && p != b'-',
        };
        if !boundary {
            continue;
        }
        if let Some(found) = numeric_at(&s[i..], min_dots) {
            return Some(found.to_string());
        }
    }
    None
}

fn git_date(token: &str) -> Option<&str> {
    let b = token.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        range
            .clone()
            .all(|i| b.get(i).is_some_and(u8::is_ascii_digit))
    };
    if b.len() >= 10
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
    {
        Some(&token[..10])
    } else {
        None
    }
}

fn nightly_build(token: &str) -> Option<&str> {
    let rest = token.strip_prefix("N-")?;
    let len = rest.bytes().take_while(u8::is_ascii_digit).count();
    (len > 0).then(|| &token[..2 + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn blender_shapes() {
        assert_eq!(
            blender("Blender 4.3.2\n\tbuild date: 2024-12-16\n\tbuild time: 23:45:21"),
            s("4.3.2")
        );
        assert_eq!(blender("Blender 4.2.0 LTS\n"), s("4.2.0"));
        assert_eq!(
            blender("Blender 3.6.5 (hash cf1e1ed46b7e built 2023-10-17 01:07:12)"),
            s("3.6.5")
        );
        assert_eq!(
            blender(
                "Color management: using fallback mode\nRead prefs: \"/Users/x/prefs.blend\"\nBlender 4.1.1\n"
            ),
            s("4.1.1")
        );
        assert_eq!(blender("Blender"), None);
        assert_eq!(blender("not blender 4.2"), None);
        assert_eq!(blender(""), None);
    }

    #[test]
    fn codex_shapes() {
        assert_eq!(codex("codex-cli 0.46.0\n"), s("0.46.0"));
        assert_eq!(codex("codex-cli 0.1.2504221401"), s("0.1.2504221401"));
        assert_eq!(codex("codex 1.2.3"), s("1.2.3"));
        assert_eq!(codex("WARNING: something\ncodex-cli 0.50.1"), s("0.50.1"));
        assert_eq!(codex("codex-cli"), None);
        assert_eq!(codex("node 1.2.3"), None);
    }

    #[test]
    fn ffmpeg_shapes() {
        assert_eq!(
            ffmpeg("ffmpeg version 7.1 Copyright (c) 2000-2024 the FFmpeg developers"),
            s("7.1")
        );
        assert_eq!(
            ffmpeg("ffmpeg version n7.0.2-20-g1234abcd Copyright"),
            s("7.0.2")
        );
        assert_eq!(
            ffmpeg("ffmpeg version 2024-11-25-git-04ce01df0b-full_build-www.gyan.dev Copyright"),
            s("git 2024-11-25")
        );
        assert_eq!(
            ffmpeg("ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023"),
            s("6.1.1")
        );
        assert_eq!(
            ffmpeg("ffmpeg version N-113684-g8a3b5c7d1e Copyright"),
            s("git N-113684")
        );
        assert_eq!(ffmpeg("ffmpeg version  "), None);
        assert_eq!(ffmpeg("garbage"), None);
    }

    #[test]
    fn toktx_shapes() {
        assert_eq!(toktx("toktx v4.3.2"), s("4.3.2"));
        assert_eq!(toktx("toktx v4.4.0~5\n"), s("4.4.0"));
        assert_eq!(toktx("toktx v4.3.0-rc1"), s("4.3.0-rc1"));
        assert_eq!(toktx("toktx v4.3.0-5-gabcdef"), s("4.3.0"));
        assert_eq!(toktx("toktx version: v4.1.0"), s("4.1.0"));
        assert_eq!(toktx("toktx"), None);
        assert_eq!(toktx("hello 4.3.2"), None);
    }

    #[test]
    fn gltf_transform_shapes() {
        assert_eq!(gltf_transform("4.1.1\n"), s("4.1.1"));
        assert_eq!(
            gltf_transform("\n  gltf-transform  \n  glTF 2.0 tools\n\n  v4.2.0\n"),
            s("4.2.0")
        );
        assert_eq!(gltf_transform("no version here 2.0"), None);
        assert_eq!(gltf_transform("abc1.2.3"), None);
    }

    #[test]
    fn node_shapes() {
        assert_eq!(node("v22.23.0\n"), s("22.23.0"));
        assert_eq!(node("22.23.0"), None);
        assert_eq!(node("vx"), None);
    }

    #[test]
    fn parse_falls_back_to_stderr() {
        assert_eq!(
            parse(ToolId::Ffmpeg, "", "ffmpeg version 7.1 Copyright"),
            s("7.1")
        );
        assert_eq!(parse(ToolId::Node, "v22.1.0", ""), s("22.1.0"));
        assert_eq!(parse(ToolId::Blender, "", ""), None);
    }
}
