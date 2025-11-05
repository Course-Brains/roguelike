use crate::log;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{LazyLock, RwLock};
static INDEX: RwLock<LazyLock<Vec<IndexEntry>>> = RwLock::new(LazyLock::new(index_initializer));
thread_local! {
    static DATA: std::cell::RefCell<std::fs::File> = std::cell::RefCell::new(std::fs::File::open("dialogue").unwrap());
}
// Gets a string from the dialogue file using the entry index defined in the index file
// This does NOT decode special characters and will crash when trying to load non ascii characters
pub fn get(index: usize) -> String {
    String::from_utf8(get_raw_bytes(index)).unwrap()
}
// Gets bytes from the dialogue file and does not touch them, good for data
pub fn get_raw_bytes(index: usize) -> Vec<u8> {
    let index_entry = INDEX.try_read().unwrap()[index];
    let mut buf = vec![0_u8; index_entry.length as usize];
    DATA.with_borrow_mut(|file| {
        file.seek(SeekFrom::Start(index_entry.start)).unwrap();
        file.read_exact(&mut buf).unwrap();
    });
    buf
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IndexEntry {
    start: u64,
    length: u64,
}
fn index_initializer() -> Vec<IndexEntry> {
    let mut file = std::io::BufReader::new(std::fs::File::open("index").unwrap());
    let mut out = Vec::new();
    let mut buf = [0; 8];

    loop {
        if let Err(error) = file.read_exact(&mut buf) {
            if let std::io::ErrorKind::UnexpectedEof = error.kind() {
                break;
            } else {
                panic!("{error}")
            }
        }
        let start = u64::from_le_bytes(buf);
        file.read_exact(&mut buf).unwrap(); // If we reach eof then something has gone wrong
        let length = u64::from_le_bytes(buf);
        out.push(IndexEntry { start, length });
    }

    out
}
pub fn editor() {
    println!("Dialogue editor session started");
    let mut index: usize = 0;
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let mut args = input.trim().split(' ');
        match args.next().unwrap() {
            "jump" => index = args.next().unwrap().parse().unwrap(),
            "show" => show(index, args.next().map(|arg| arg.parse().unwrap())),
            "next" => {
                if index + 1 >= std::fs::metadata("index").unwrap().len() as usize / 16 {
                    println!("End of indexes");
                } else {
                    index += 1;
                    println!("Now at {index}");
                }
            }
            "prev" => {
                if index == 0 {
                    println!("Start of indexes");
                } else {
                    index -= 1;
                    println!("Now at {index}");
                }
            }
            "count" => count(),
            "metadata" => metadata(index),
            "set" => set(index, args.next()),
            "add_new" | "new" | "add" => add_new(&mut index, args.next()),
            "full_reset" => full_reset(&mut index),
            "help" => help(),
            "quit" => return,
            _ => help(),
        }
        *INDEX.try_write().unwrap() = LazyLock::new(index_initializer);
    }
}
fn show(index: usize, mode: Option<ShowMode>) {
    if let Some(mode) = mode {
        match mode {
            ShowMode::Hex => {
                let bytes = get_raw_bytes(index);
                for byte in bytes.iter() {
                    print!("{byte:2x} ");
                }
            }
        }
        println!();
    } else {
        println!("{}: \"{}\"", index, get(index));
    }
}
fn count() {
    println!(
        "There are {} entries",
        std::fs::File::open("index")
            .unwrap()
            .metadata()
            .unwrap()
            .len()
            / 16
    );
}
fn add_new(index: &mut usize, path: Option<&str>) {
    *index = std::fs::metadata("index").unwrap().len() as usize / 16;
    if path.is_none() {
        std::process::Command::new("vim")
            .arg("new_dialogue")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        if !std::fs::exists("new_dialogue").unwrap() {
            return;
        }
    }
    let path = path.unwrap_or("new_dialogue");
    let mut text = encode(std::fs::read(path).unwrap());
    text.pop();
    std::fs::remove_file(path).unwrap();

    let start = std::fs::metadata("dialogue").unwrap().len();
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(false)
        .append(true)
        .open("dialogue")
        .unwrap()
        .write_all(&text)
        .unwrap();
    let mut index_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(false)
        .append(true)
        .open("index")
        .unwrap();
    index_file.write_all(&start.to_le_bytes()).unwrap();
    index_file
        .write_all(&(text.len() as u64).to_le_bytes())
        .unwrap();
    println!(
        "Added \"{}\" to new index {index}",
        String::from_utf8_lossy(&text)
    );
}
fn set(index: usize, path: Option<&str>) {
    log!("Setting {index}");
    let old = INDEX.try_read().unwrap()[index];
    if path.is_none() {
        std::fs::write("new_dialogue", get(index).as_bytes()).unwrap();
        std::process::Command::new("vim")
            .arg("new_dialogue")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }
    let path = path.unwrap_or("new_dialogue");
    let mut text = encode(std::fs::read(path).unwrap());
    text.pop();
    let new_length = text.len() as u64;
    log!("New dialogue has length: {new_length}");
    if new_length != old.length {
        // Size has changed :(

        log!("Have to move later entries");
        let difference = new_length as i64 - old.length as i64;
        log!("difference is {difference}");

        // Shift the data
        let mut data_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .open("dialogue")
            .unwrap();

        // Copy data from old_end_of_data..eof to new_end_of_data..eof+difference
        let mut buf = Vec::new();
        data_file
            .seek(SeekFrom::Start(old.start + old.length))
            .unwrap();
        data_file.read_to_end(&mut buf).unwrap();
        log!("Copying \"{}\"", String::from_utf8(buf.clone()).unwrap());
        data_file
            .seek(SeekFrom::Start(old.start + old.length))
            .unwrap();
        data_file.seek(SeekFrom::Current(difference)).unwrap();
        data_file.write_all(&buf).unwrap();

        // Shift the start points
        let mut buf = [0_u8; 8];
        let mut index_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .open("index")
            .unwrap();
        log!("Iterating through entries to modify starts");
        loop {
            if let Err(error) = index_file.read_exact(&mut buf) {
                if let std::io::ErrorKind::UnexpectedEof = error.kind() {
                    break;
                } else {
                    panic!("{error}");
                }
            }
            let start = u64::from_le_bytes(buf);
            log!("  read start pos: {start}");
            if start > old.start {
                log!("Changing start pos to {}", start as i64 + difference);
                index_file.seek(SeekFrom::Current(-8)).unwrap();
                index_file
                    .write_all(&((start as i64 + difference) as u64).to_le_bytes())
                    .unwrap();
            }
            index_file.seek(SeekFrom::Current(8)).unwrap();
        }
        // If it shrunk then we need to remove the extra data from the file
        if new_length < old.length {
            data_file
                .set_len((data_file.metadata().unwrap().len() as i64 + difference) as u64)
                .unwrap();
        }
    }
    let mut index_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(false)
        .open("index")
        .unwrap();
    // Updating current index entry
    index_file
        .seek(SeekFrom::Start((index as u64 * 16) + 8))
        .unwrap();
    // +8 because we don't need to change the start
    index_file.write_all(&new_length.to_le_bytes()).unwrap();
    log!(
        "Writing new text \"{}\" to dialogue file",
        String::from_utf8_lossy(&text)
    );
    std::fs::remove_file(path).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(false)
        .open("dialogue")
        .unwrap();
    file.seek(SeekFrom::Start(old.start)).unwrap();
    file.write_all(&text).unwrap();
}
fn help() {
    println!(
        "Valid commands are: jump [index], show, next, prev, count, \
        metadata, set, add_new, full_reset, help, quit"
    );
}
fn full_reset(index: &mut usize) {
    if abes_nice_things::Input::<()>::yn()
        .msg("Are you sure?")
        .get()
        .unwrap()
        .as_str()
        == "y"
    {
        *index = 0;
        let test_msg = "Testing... YIPPY!!!".to_string();
        std::fs::File::create("dialogue")
            .unwrap()
            .write_all(test_msg.as_bytes())
            .unwrap();
        let mut index = std::fs::File::create("index").unwrap();
        index.write_all(&0_u64.to_le_bytes()).unwrap();
        index
            .write_all(&(test_msg.len() as u64).to_le_bytes())
            .unwrap();
    }
}
fn metadata(index: usize) {
    let entry = INDEX.try_read().unwrap()[index];
    println!(
        "Entry number {index} starts at {} and is {} long",
        entry.start, entry.length
    );
}
fn encode(string: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(string.len());
    let mut iter = string.into_iter();
    while let Some(ch) = iter.next() {
        if ch != b'\\' {
            out.push(ch);
            continue;
        }
        let mut code = Vec::new();
        if let Some(next) = iter.next() {
            code.push(next);
            if let Some(next) = iter.next() {
                code.push(next);
                if let Ok(string) = String::from_utf8(code) {
                    if let Ok(code) = u8::from_str_radix(&string, 16) {
                        out.push(code);
                    }
                }
            }
        }
    }
    out
}
enum ShowMode {
    Hex,
}
impl std::str::FromStr for ShowMode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "hex" => Ok(Self::Hex),
            _ => Err(()),
        }
    }
}
