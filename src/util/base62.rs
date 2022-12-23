use rand::distributions::Uniform;
use rand::Rng;

const CHARS: [char; 62] = [
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l',
    'z', 'x', 'c', 'v', 'b', 'n', 'm', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', 'A', 'S',
    'D', 'F', 'G', 'H', 'J', 'K', 'L', 'Z', 'X', 'C', 'V', 'B', 'N', 'M', '1', '2', '3', '4', '5',
    '6', '7', '8', '9', '0',
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
