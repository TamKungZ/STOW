use crate::{codec, fold, rc, Error, Reader, Writer};

fn lcg_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state >> 56) as u8
        })
        .collect()
}

fn samples() -> Vec<Vec<u8>> {
    let mut text = Vec::new();
    for i in 0..400 {
        text.extend_from_slice(format!("line {} the quick brown fox jumps over the lazy dog\n", i % 17).as_bytes());
    }
    vec![
        Vec::new(),
        vec![7],
        vec![1, 2],
        vec![0; 1000],
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec(),
        b"abababababababababababab".to_vec(),
        b"A84R5A84R5A84R5A84R5A84R5".to_vec(),
        text,
        lcg_bytes(5000, 1),
        (0..=255u8).cycle().take(4096).collect(),
    ]
}

#[test]
fn fold_roundtrip() {
    for s in samples() {
        let enc = fold::encode(&s);
        assert_eq!(fold::decode(&enc, s.len()).unwrap(), s);
    }
}

#[test]
fn rc_roundtrip() {
    for s in samples() {
        let enc = rc::encode(&s);
        assert_eq!(rc::decode(&enc, s.len()).unwrap(), s);
    }
}

#[test]
fn every_codec_roundtrip() {
    for s in samples() {
        for (id, payload) in codec::candidates(&s) {
            assert_eq!(codec::decode(id, &payload, s.len()).unwrap(), s, "codec {}", id);
        }
    }
}

#[test]
fn smallest_never_exceeds_store() {
    for s in samples() {
        let (_, payload) = codec::smallest(&s);
        assert!(payload.len() <= s.len());
    }
}

#[test]
fn container_roundtrip() {
    let all = samples();
    let mut w = Writer::new();
    for (i, s) in all.iter().enumerate() {
        w.add(&format!("dir/file{}.bin", i), s).unwrap();
    }
    let bytes = w.finish();
    let r = Reader::new(bytes).unwrap();
    assert_eq!(r.len(), all.len());
    for (i, s) in all.iter().enumerate() {
        assert_eq!(r.entries()[i].name, format!("dir/file{}.bin", i));
        assert_eq!(&r.read(i).unwrap(), s);
    }
    assert_eq!(r.read(all.len()).unwrap_err(), Error::IndexOutOfRange);
}

#[test]
fn empty_archive() {
    let bytes = Writer::new().finish();
    let r = Reader::new(bytes).unwrap();
    assert!(r.is_empty());
}

#[test]
fn rejects_bad_names() {
    let mut w = Writer::new();
    for bad in ["", "/abs", "../x", "a/../b", "a//b", "a/", "./a", "a\\b", "C:x", "a\0b"] {
        assert_eq!(w.add(bad, b"x").unwrap_err(), Error::InvalidName, "{:?}", bad);
    }
    w.add("ok/name.txt", b"x").unwrap();
    assert_eq!(w.add("ok/name.txt", b"y").unwrap_err(), Error::DuplicateName);
}

#[test]
fn detects_corruption() {
    let mut w = Writer::new();
    w.add("a.txt", &samples()[7]).unwrap();
    let good = w.finish();

    assert_eq!(Reader::new(b"nope".to_vec()).err(), Some(Error::InvalidMagic));
    assert!(Reader::new(good[..good.len() - 1].to_vec()).is_err());

    let mut flipped_any = false;
    for i in 10..good.len() {
        let mut bad = good.clone();
        bad[i] ^= 0x55;
        if let Ok(r) = Reader::new(bad) {
            if r.read(0).is_err() {
                flipped_any = true;
            }
        }
    }
    assert!(flipped_any);
}

#[test]
fn garbage_never_panics() {
    for seed in 0..300u64 {
        let junk = lcg_bytes(64 + (seed as usize % 200), seed);
        for id in 0..4u8 {
            let _ = codec::decode(id, &junk, 1000);
        }
        let mut with_magic = b"STOW\x01\x00".to_vec();
        with_magic.extend_from_slice(&junk);
        if let Ok(r) = Reader::new(with_magic) {
            for i in 0..r.len() {
                let _ = r.read(i);
            }
        }
    }
}

#[test]
fn expansion_is_bounded() {
    let mut src = Vec::new();
    crate::varint::put(&mut src, 40);
    crate::varint::put(&mut src, 1);
    for i in 0..40u64 {
        crate::varint::put(&mut src, if i == 0 { 0 } else { 255 + i });
        crate::varint::put(&mut src, if i == 0 { 0 } else { 255 + i });
    }
    crate::varint::put(&mut src, 255 + 40);
    assert_eq!(fold::decode(&src, 100).unwrap_err(), Error::Corrupt);
}
