use crate::tmplayer::app::state::{LyricLine, TrackMetadata};
use anyhow::Result;
use lofty::file::{AudioFile, TaggedFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

const MAX_LOCAL_COVER_BYTES: u64 = 8 * 1024 * 1024;

pub fn read_metadata(path: &Path) -> Result<TrackMetadata> {
    let mut meta = TrackMetadata::default();

    let tagged = lofty::read_from_path(path)?;
    let properties = tagged.properties();
    meta.duration = properties.duration();

    if let Some(tag) = tagged.primary_tag() {
        if let Some(t) = tag.title() {
            meta.title = t.to_string();
        }
        if let Some(a) = tag.artist() {
            meta.artist = a.to_string();
        }
        if let Some(al) = tag.album() {
            meta.album = al.to_string();
        }
    }

    // Embedded cover (prefer any embedded picture across all tags; best-effort)
    if meta.cover.is_none() {
        if let Some((bytes, hash)) = read_embedded_cover(&tagged) {
            meta.cover_hash = Some(hash);
            meta.cover = Some(bytes);
            meta.cover_folder = path.parent().map(|p| p.to_path_buf());
        }
    }

    // Per-track cover in subfolder: <dir>/cover/<stem>.(jpg|png)
    if meta.cover.is_none() {
        if let Some((bytes, hash)) = read_cover_for_audio(path) {
            meta.cover_hash = Some(hash);
            meta.cover = Some(bytes);
            meta.cover_folder = path.parent().map(|p| p.to_path_buf());
        }
    }

    // Fallback: local folder cover image near the audio file.
    if meta.cover.is_none() {
        let folder = path.parent().unwrap_or(Path::new("."));
        if let Some((bytes, hash)) = read_cover_from_folder(folder) {
            meta.cover_hash = Some(hash);
            meta.cover = Some(bytes);
            meta.cover_folder = Some(folder.to_path_buf());
        }
    }

    // fallback title from filename
    if meta.title == "Unknown" {
        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
            meta.title = name.to_string();
        }
    }

    // Embedded lyrics first; fallback to local .lrc.
    meta.lyrics = read_embedded_lyrics(&tagged).or_else(|| read_lrc_for_audio(path));

    Ok(meta)
}

fn read_embedded_cover(tagged: &TaggedFile) -> Option<(Vec<u8>, u64)> {
    // Try primary tag first, then other tags.
    if let Some(t) = tagged.primary_tag() {
        if let Some((b, h)) = read_cover_from_tag(t) {
            return Some((b, h));
        }
    }
    for t in tagged.tags() {
        if let Some((b, h)) = read_cover_from_tag(t) {
            return Some((b, h));
        }
    }
    None
}

fn read_cover_from_tag(tag: &Tag) -> Option<(Vec<u8>, u64)> {
    let pic = tag.pictures().first()?;
    let bytes = pic.data();
    if bytes.is_empty() || bytes.len() as u64 > MAX_LOCAL_COVER_BYTES {
        return None;
    }
    let hash = hash_bytes(bytes);
    Some((bytes.to_vec(), hash))
}

pub fn read_cover_from_folder(dir: &Path) -> Option<(Vec<u8>, u64)> {
    // Common filenames used by many players.
    // Keep this list small and predictable.
    let candidates = [
        "cover", "folder", "front", "album", "artwork", "Cover", "Folder", "Front",
    ];
    let exts = ["jpg", "jpeg", "png"];

    for base in candidates {
        for ext in exts {
            let p = dir.join(format!("{base}.{ext}"));
            if let Some(cover) = read_cover_file(&p) {
                return Some(cover);
            }
        }
    }
    None
}

fn read_cover_for_audio(audio_path: &Path) -> Option<(Vec<u8>, u64)> {
    let Some(folder) = audio_path.parent() else {
        return None;
    };
    let Some(stem) = audio_path.file_stem().and_then(|s| s.to_str()) else {
        return None;
    };
    let exts = ["jpg", "jpeg", "png"];
    let cover_dir = folder.join("cover");
    for ext in exts {
        let p = cover_dir.join(format!("{stem}.{ext}"));
        if let Some(cover) = read_cover_file(&p) {
            return Some(cover);
        }
    }
    None
}

fn read_cover_file(path: &Path) -> Option<(Vec<u8>, u64)> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }

    let len = metadata.len();
    if len == 0 || len > MAX_LOCAL_COVER_BYTES {
        return None;
    }

    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }

    let hash = hash_bytes(&bytes);
    Some((bytes, hash))
}

fn read_embedded_lyrics(tagged: &TaggedFile) -> Option<Vec<LyricLine>> {
    // Try primary tag first, then other tags.
    if let Some(t) = tagged.primary_tag() {
        if let Some(lines) = read_lyrics_from_tag(t) {
            return Some(lines);
        }
    }
    for t in tagged.tags() {
        if let Some(lines) = read_lyrics_from_tag(t) {
            return Some(lines);
        }
    }
    None
}

fn read_lyrics_from_tag(tag: &Tag) -> Option<Vec<LyricLine>> {
    let raw = tag.get_string(ItemKey::Lyrics)?.trim();
    if raw.is_empty() {
        return None;
    }

    // If it's LRC-like, parse timestamps.
    if let Some(parsed) = parse_lrc(raw) {
        return Some(parsed);
    }

    // Otherwise treat it as unsynchronized lyrics: show first 1-2 lines statically.
    let mut non_empty = raw.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = non_empty.next()?.to_string();
    let second = non_empty.next().map(|s| s.to_string());

    let mut out = Vec::new();
    out.push(LyricLine {
        start_ms: 0,
        text: first,
        translation: None,
    });
    if let Some(s2) = second {
        out.push(LyricLine {
        start_ms: u64::MAX,
        text: s2,
        translation: None,
    });
    }
    Some(out)
}

fn read_lrc_for_audio(audio_path: &Path) -> Option<Vec<LyricLine>> {
    let mut candidates = Vec::new();
    candidates.push(audio_path.with_extension("lrc"));

    if let (Some(folder), Some(stem)) = (
        audio_path.parent(),
        audio_path.file_stem().and_then(|s| s.to_str()),
    ) {
        candidates.push(folder.join("lrc").join(format!("{stem}.lrc")));
    }

    for p in candidates {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Some(lines) = parse_lrc(&content).or_else(|| parse_plain_lyrics(&content)) {
                return Some(lines);
            }
        }
    }
    None
}

const LYRIC_CREDIT_PREFIXES: &[&str] = &[
    // 简体/繁体中文
    "作词", "作詞", "填词", "填詞", "作曲", "编曲", "編曲", "制作人", "製作人", "制作", "製作",
    "词曲", "詞曲", "演唱", "歌手", "原唱", "翻唱", "混音", "混音师", "混音師", "母带", "母帶",
    "母带工程师", "母帶工程師", "录音", "錄音", "录音师", "錄音師", "和声", "和聲", "和音",
    "配唱", "配唱制作人", "监制", "監製", "音乐监制", "音乐总监", "出品", "出品人", "发行",
    "發行", "策划", "策劃", "企划", "企劃", "统筹", "統籌", "版权", "版權", "鸣谢", "鳴謝",
    "特别鸣谢", "特別鳴謝", "翻译", "翻譯", "封面", "设计", "经纪", "文案", "企宣",
    "吉他", "贝斯", "貝斯", "鼓", "键盘", "鍵盤", "弦乐", "弦樂", "钢琴", "大提琴", "小提琴",
    "中提琴", "词", "詞", "曲",
    // 英文
    "Lyrics by", "Written by", "Composed by", "Arranged by", "Produced by", "Performed by",
    "Music by", "Words by", "Vocals by", "Mastered by", "Mixed by", "Recorded by",
    "Published by", "Lyricist", "Composer", "Arranger", "Producer", "Mixing Engineer",
    "Mastering Engineer", "Recording Engineer", "Vocal Producer", "Executive Producer",
    "Background Vocals", "Backing Vocals", "Guitar", "Bass", "Drums", "Keyboard", "Strings",
    "Piano", "Cello", "Violin", "Viola", "OP", "SP",
];

pub fn is_lyric_credit_line(text: &str) -> bool {
    let plain = text.trim_start();
    LYRIC_CREDIT_PREFIXES.iter().any(|prefix| {
        let Some(head) = plain.get(..prefix.len()) else {
            return false;
        };
        if !head.eq_ignore_ascii_case(prefix) {
            return false;
        }
        let rest = plain[prefix.len()..].trim_start_matches(' ');
        rest.is_empty()
            || rest.starts_with([':', '：', '/', '／', '-'])
            || prefix.ends_with(" by")
    })
}

pub fn parse_lrc(content: &str) -> Option<Vec<LyricLine>> {
    let mut out: Vec<LyricLine> = Vec::new();

    for raw in content.lines() {
        let mut s = raw.trim();
        if s.is_empty() {
            continue;
        }

        // Collect leading [..] tags; keep all time tags, ignore metadata tags like [ti:]
        let mut times: Vec<u64> = Vec::new();
        while let Some(rest) = s.strip_prefix('[') {
            let Some(end) = rest.find(']') else {
                break;
            };
            let tag = &rest[..end];
            if let Some(ms) = parse_lrc_time_tag(tag) {
                times.push(ms);
            }
            s = &rest[end + 1..];
        }

        if times.is_empty() {
            continue;
        }

        let text = s.trim().to_string();
        if is_lyric_credit_line(&text) {
            continue;
        }
        for t in times {
            out.push(LyricLine {
                start_ms: t,
                text: text.clone(),
                translation: None,
            });
        }
    }

    if out.is_empty() {
        return None;
    }
    out.sort_by_key(|l| l.start_ms);
    Some(out)
}

pub fn parse_plain_lyrics(content: &str) -> Option<Vec<LyricLine>> {
    let mut non_empty = content.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = non_empty.next()?.to_string();
    let second = non_empty.next().map(|s| s.to_string());

    let mut out = Vec::new();
    out.push(LyricLine {
        start_ms: 0,
        text: first,
        translation: None,
    });
    if let Some(s2) = second {
        out.push(LyricLine {
        start_ms: u64::MAX,
        text: s2,
        translation: None,
    });
    }
    Some(out)
}

fn parse_lrc_time_tag(tag: &str) -> Option<u64> {
    // Supports mm:ss, mm:ss.xx, mm:ss.xxx
    // Rejects metadata tags like "ti:xxx" by requiring numeric mm and ss.
    let (mm_s, rest) = tag.split_once(':')?;
    let mm: u64 = mm_s.trim().parse().ok()?;

    let rest = rest.trim();
    let (ss_s, frac_s) = if let Some((a, b)) = rest.split_once('.') {
        (a, Some(b))
    } else {
        (rest, None)
    };
    let ss: u64 = ss_s.trim().parse().ok()?;
    if ss >= 60 {
        // be lenient but avoid obvious non-timestamps
        return None;
    }

    let mut ms: u64 = 0;
    if let Some(frac) = frac_s {
        let frac = frac.trim();
        let digits: String = frac
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .take(3)
            .collect();
        if digits.is_empty() {
            ms = 0;
        } else if digits.len() == 1 {
            ms = digits.parse::<u64>().ok()? * 100;
        } else if digits.len() == 2 {
            ms = digits.parse::<u64>().ok()? * 10;
        } else {
            ms = digits.parse::<u64>().ok()?;
        }
    }

    Some(mm * 60_000 + ss * 1_000 + ms)
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_lines_are_detected() {
        for line in [
            "作词 : 林夕",
            "作曲：泽日生",
            "编曲 / 陈辉阳",
            "制作人: 陈辉阳",
            "OP/SP：某某",
            "Lyrics by John",
            "Composer: Smith",
            "混音师 - 张三",
        ] {
            assert!(is_lyric_credit_line(line), "should filter: {line}");
        }
    }

    #[test]
    fn normal_lyrics_are_kept() {
        for line in [
            "作词人的寂寞",
            "海边",
            "OP 到底是什么",
            "Bass 低音响起",
            "Normal lyric line",
        ] {
            assert!(!is_lyric_credit_line(line), "should keep: {line}");
        }
    }
}
