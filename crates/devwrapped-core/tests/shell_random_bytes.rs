use devwrapped_core::shell::{
    HistoryScan, Installed, bash, fish, parse_line, powershell, typos, zsh,
};

const INPUTS: usize = 2_000;
const MAX_LEN: usize = 4_096;
const FORMAT_BYTES: &[u8] = b": ;#\\`-\r\n0123456789cmdwhen\xff\xef\xbb\xbf";
const LINE_STARTS: &[&[u8]] = &[
    b"- cmd: ",
    b"  when: 1767225600",
    b"  when: ",
    b"  paths:",
    b"    - ",
    b": 1767225600:0;",
    b": ",
    b"#1767225600",
    b"#",
    b"",
];

// a fixed seed keeps every run on the same inputs, so a failure can be reproduced
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next() % limit as u64) as usize
    }

    // half the bytes come from the format syntax so the parsers reach their deeper branches
    fn bytes(&mut self) -> Vec<u8> {
        let len = self.below(MAX_LEN);
        (0..len)
            .map(|_| match self.below(2) {
                0 => FORMAT_BYTES[self.below(FORMAT_BYTES.len())],
                _ => self.next() as u8,
            })
            .collect()
    }

    // lines that start like real entries reach the parts of each parser that random bytes miss
    fn lines(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        for _ in 0..self.below(64) {
            out.extend_from_slice(LINE_STARTS[self.below(LINE_STARTS.len())]);
            let len = self.below(48);
            out.extend((0..len).map(|_| match self.below(3) {
                0 => FORMAT_BYTES[self.below(FORMAT_BYTES.len())],
                1 => b"git cargo ls docker "[self.below(20)],
                _ => self.next() as u8,
            }));
            out.push(b'\n');
        }
        out
    }
}

fn check(scan: HistoryScan) -> usize {
    for command in &scan.commands {
        let _ = typos::find_typo(command);
    }
    scan.commands.len()
}

#[test]
fn parsers_never_panic_on_random_bytes() {
    let mut random = XorShift(0x5eed_1234_abcd_0001);
    let mut parsed = [0; 4];
    for round in 0..INPUTS {
        let input = if round % 2 == 0 {
            random.bytes()
        } else {
            random.lines()
        };
        parsed[0] += check(zsh::parse(&input[..], &Installed::default()));
        parsed[1] += check(bash::parse(&input[..], &Installed::default()));
        parsed[2] += check(fish::parse(&input[..], &Installed::default()));
        parsed[3] += check(powershell::parse(&input[..], &Installed::default()));
        let _ = parse_line(
            &String::from_utf8_lossy(&input),
            None,
            &Installed::default(),
        );
    }
    assert!(parsed.iter().all(|&count| count > 0), "{parsed:?}");
}
