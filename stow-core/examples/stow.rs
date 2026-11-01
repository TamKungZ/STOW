use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stow_core::{codec, Reader, Writer};

fn join(base: &str, part: &str) -> String {
    if base.is_empty() {
        part.to_owned()
    } else {
        format!("{}/{}", base, part)
    }
}

fn walk(w: &mut Writer, path: &Path, name: String) -> Result<(), String> {
    if path.is_dir() {
        let mut children: Vec<_> = fs::read_dir(path)
            .map_err(|e| format!("{}: {}", path.display(), e))?
            .filter_map(|e| e.ok())
            .collect();
        children.sort_by_key(|e| e.file_name());
        for child in children {
            let child_name = join(&name, &child.file_name().to_string_lossy());
            walk(w, &child.path(), child_name)?;
        }
        Ok(())
    } else {
        let data = fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        w.add(&name, &data).map_err(|e| format!("{}: {}", name, e))
    }
}

fn base_name(arg: &str) -> String {
    Path::new(arg)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn create(out: &str, inputs: &[String]) -> Result<(), String> {
    let mut w = Writer::new();
    for input in inputs {
        walk(&mut w, Path::new(input), base_name(input))?;
    }
    fs::write(out, w.finish()).map_err(|e| format!("{}: {}", out, e))
}

fn open(path: &str) -> Result<Reader, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {}", path, e))?;
    Reader::new(bytes).map_err(|e| format!("{}: {}", path, e))
}

fn list(path: &str) -> Result<(), String> {
    let r = open(path)?;
    for e in r.entries() {
        println!("{:>12} {:>12} {:<8} {}", e.raw_len, e.stored_len, codec::name(e.codec), e.name);
    }
    Ok(())
}

fn extract(path: &str, dir: &str) -> Result<(), String> {
    let r = open(path)?;
    for (i, e) in r.entries().iter().enumerate() {
        let data = r.read(i).map_err(|err| format!("{}: {}", e.name, err))?;
        let target: PathBuf = e.name.split('/').fold(PathBuf::from(dir), |p, s| p.join(s));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("{}: {}", parent.display(), err))?;
        }
        fs::write(&target, data).map_err(|err| format!("{}: {}", target.display(), err))?;
    }
    Ok(())
}

fn stats(inputs: &[String]) -> Result<(), String> {
    println!("{:>12} {:>12} {:>12} {:>12} {:>12}  {}", "raw", "fold+rc", "fold+ac", "xform", "chosen", "file");
    for input in inputs {
        let data = fs::read(input).map_err(|e| format!("{}: {}", input, e))?;
        let all = codec::candidates(&data);
        let size = |id: u8| {
            all.iter()
                .find(|(i, _)| *i == id)
                .map(|(_, p)| p.len().to_string())
                .unwrap_or_else(|| "-".into())
        };
        let xform = all
            .iter()
            .filter(|(i, _)| *i == codec::XFORM)
            .map(|(_, p)| p.len())
            .min()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into());
        let (chosen, payload) = codec::smallest(&data);
        println!(
            "{:>12} {:>12} {:>12} {:>12} {:>12}  {} ({})",
            data.len(),
            size(codec::FOLD_RC),
            size(codec::FOLD_AC),
            xform,
            payload.len(),
            input,
            codec::describe(chosen, &payload)
        );
    }
    Ok(())
}

fn usage() -> ExitCode {
    eprintln!("usage:\n  stow create <out.sto> <path>...\n  stow list <in.sto>\n  stow extract <in.sto> <dir>\n  stow stats <file>...");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("create") if args.len() >= 3 => create(&args[1], &args[2..]),
        Some("list") if args.len() == 2 => list(&args[1]),
        Some("extract") if args.len() == 3 => extract(&args[1], &args[2]),
        Some("stats") if args.len() >= 2 => stats(&args[1..]),
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {}", msg);
            ExitCode::FAILURE
        }
    }
}
