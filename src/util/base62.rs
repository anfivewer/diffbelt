use rand::distributions::Uniform;
use rand::Rng;

const CHARS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b',
    'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'x', 'y', 'z',
];

pub fn rand_b62(len: usize) -> String {
    let mut result = String::with_capacity(len);

    let mut rng = rand::thread_rng();
    let between = Uniform::from(0..62usize);

    for _ in 0..len {
        let i = rng.sample(&between);
        result.push(CHARS[i]);
    }

    result
}

pub fn from_u64(mut value: u64) -> Box<str> {
    let mut boxed: Box<[u8]> = Box::from([0u8; 11]);
    let s = boxed.as_mut();

    for i in 0..11 {
        let char_i = 10usize - i;

        let n = value % 62;
        value /= 62;

        s[char_i] = CHARS[n as usize] as u8;
    }

    unsafe { std::mem::transmute::<Box<[u8]>, Box<str>>(boxed) }
}

fn char_byte_as_b62(byte: u8) -> Result<u8, ()> {
    if 48 <= byte && byte <= 57 {
        return Ok(byte - 48);
    }

    if 65 <= byte && byte <= 90 {
        return Ok(10 + byte - 65);
    }

    if 97 <= byte && byte <= 122 {
        return Ok(10 + 26 + byte - 97);
    }

    Err(())
}

pub fn to_u64(s: &str) -> Result<u64, ()> {
    let bytes = s.as_bytes();
    let len = bytes.len();

    let mut result = char_byte_as_b62(bytes[0])? as u64;

    for i in 1..len {
        result = result.checked_mul(62).ok_or(())?;

        let n = char_byte_as_b62(bytes[i])?;

        result = result.checked_add(n as u64).ok_or(())?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::util::base62::{from_u64, to_u64};

    #[test]
    fn test_from_u64() {
        assert_eq!(from_u64(0).as_ref(), "00000000000");
        assert_eq!(from_u64(1).as_ref(), "00000000001");
        assert_eq!(from_u64(62).as_ref(), "00000000010");
        assert_eq!(from_u64(0xffffffffffffffff).as_ref(), "LygHa16AHYF");
    }

    #[test]
    fn test_to_u64() {
        assert_eq!(to_u64("0").unwrap(), 0);
        assert_eq!(to_u64("1").unwrap(), 1);
        assert_eq!(to_u64("0001").unwrap(), 1);
        assert_eq!(to_u64("LygHa16AHYF").unwrap(), 0xffffffffffffffff);
        assert!(to_u64("LygHa16AHYG").is_err());
        assert!(to_u64("zzzzzzzzzzz").is_err());
    }
}
