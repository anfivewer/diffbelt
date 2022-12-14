use std::fs::{create_dir, remove_dir_all};
use std::io::ErrorKind;
use std::path::PathBuf;

pub struct TempDir(PathBuf);

const CHARS_COUNT: usize = 8;
const PREFIX: &str = "diffbelt_test_";
const A_ORD: u32 = 'a' as u32;

impl TempDir {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp_dir = std::env::temp_dir();

        let mut str = String::with_capacity(PREFIX.len() + CHARS_COUNT);

        loop {
            str.clear();
            str.push_str(PREFIX);

            for _ in 0..CHARS_COUNT {
                let rand = rand::random::<u32>();
                let val = (rand as f64) / ((u32::MAX as f64) + 1.0);
                let val = (val * 26.0).floor() as u32;
                let chr = unsafe { char::from_u32_unchecked(A_ORD + val) };
                str.push(chr);
            }

            let joined = temp_dir.join(PathBuf::from(&str));

            let result = create_dir(joined.as_path());

            match result {
                Ok(_) => {
                    return Ok(Self(joined));
                }
                Err(err) => match err.kind() {
                    ErrorKind::AlreadyExists => {}
                    _ => {
                        return Err(err);
                    }
                },
            }
        }
    }

    pub fn get_path_buf(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        remove_dir_all(self.0.as_path()).unwrap_or(());
    }
}
