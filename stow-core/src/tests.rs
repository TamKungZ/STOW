use crate::{ac, codec, fold, rc, transform, Error, Reader, Writer};

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
        for id in 0..6u8 {
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

#[test]
fn ac_survives_mutation() {
    let s = &samples()[7];
    let enc = ac::encode(&fold::build(s));
    assert_eq!(ac::decode(&enc, s.len()).unwrap(), *s);
    assert!(ac::decode(&enc[..enc.len() - 1], s.len()).is_err());
    assert!(ac::decode(&enc, s.len() + 1).is_err());
    assert!(ac::decode(&enc, s.len() - 1).is_err());
    let mut failed = 0;
    for i in 0..enc.len() {
        let mut bad = enc.clone();
        bad[i] ^= 0x55;
        match ac::decode(&bad, s.len()) {
            Ok(out) => assert_eq!(out.len(), s.len()),
            Err(_) => failed += 1,
        }
    }
    assert!(failed > 0);
}

#[test]
fn ac_counts_are_bounded() {
    let mut src = Vec::new();
    crate::varint::put(&mut src, u64::MAX);
    crate::varint::put(&mut src, 1);
    src.extend_from_slice(&[0; 16]);
    assert_eq!(ac::decode(&src, 100).unwrap_err(), Error::Corrupt);

    let mut src = Vec::new();
    crate::varint::put(&mut src, 1 << 20);
    crate::varint::put(&mut src, 1 << 20);
    src.extend_from_slice(&[0; 8]);
    assert!(ac::decode(&src, 100).is_err());
}

#[test]
fn ac_modes_roundtrip() {
    for s in samples() {
        let g = fold::build(&s);
        for mode in [ac::ABSOLUTE, ac::DISTANCE] {
            let enc = ac::encode_with(&g, mode);
            assert_eq!(ac::decode(&enc, s.len()).unwrap(), s, "mode {}", mode);
            let mut bad = enc.clone();
            bad[0] = 2;
            assert_eq!(ac::decode(&bad, s.len()).unwrap_err(), Error::Corrupt);
        }
    }
    assert_eq!(ac::decode(&[], 0).unwrap_err(), Error::Truncated);
}

fn numeric_samples() -> Vec<Vec<u8>> {
    let mono: Vec<u8> = (0..3000u32).flat_map(|i| (i * 7 + i % 5).to_le_bytes()).collect();
    let sine: Vec<u8> = (0..4000)
        .flat_map(|i| ((8000.0 * (i as f64 * 0.02).sin()) as i16).to_le_bytes())
        .collect();
    let mut odd = mono.clone();
    odd.extend_from_slice(&[1, 2, 3]);
    vec![mono, sine, odd]
}

#[test]
fn transforms_invert() {
    for s in samples().into_iter().chain(numeric_samples()) {
        for id in transform::ALL {
            let mut moved = transform::forward(id, &s);
            assert_eq!(moved.len(), s.len());
            transform::inverse(id, &mut moved).unwrap();
            assert_eq!(moved, s, "transform {}", id);
        }
    }
    assert!(transform::inverse(0, &mut [0; 4]).is_err());
    assert!(transform::inverse(7, &mut [0; 4]).is_err());
}

#[test]
fn xform_roundtrip_and_wins_on_numbers() {
    for s in samples().into_iter().chain(numeric_samples()) {
        for (id, payload) in codec::candidates(&s) {
            assert_eq!(codec::decode(id, &payload, s.len()).unwrap(), s, "codec {}", id);
        }
    }
    let mono = &numeric_samples()[0];
    let (id, payload) = codec::smallest(mono);
    assert_eq!(id, codec::XFORM);
    assert_eq!(codec::decode(id, &payload, mono.len()).unwrap(), *mono);
}

#[test]
fn xform_rejects_bad_payloads() {
    let s = &numeric_samples()[0];
    let (id, payload) = codec::smallest(s);
    assert_eq!(id, codec::XFORM);
    assert_eq!(codec::decode(codec::XFORM, &[], 10).unwrap_err(), Error::Truncated);
    assert_eq!(codec::decode(codec::XFORM, &[1], 10).unwrap_err(), Error::Truncated);
    assert_eq!(codec::decode(codec::XFORM, &[0, codec::STORE, 1], 1).unwrap_err(), Error::UnknownCodec);
    assert_eq!(codec::decode(codec::XFORM, &[9, codec::STORE, 1], 1).unwrap_err(), Error::UnknownCodec);
    assert_eq!(codec::decode(codec::XFORM, &[1, codec::XFORM, 1], 1).unwrap_err(), Error::UnknownCodec);
    assert!(codec::decode(codec::XFORM, &payload[..payload.len() - 1], s.len()).is_err());
    assert!(codec::decode(codec::XFORM, &payload, s.len() + 1).is_err());
    for i in 0..payload.len() {
        let mut bad = payload.clone();
        bad[i] ^= 0x55;
        let _ = codec::decode(codec::XFORM, &bad, s.len());
    }
}
