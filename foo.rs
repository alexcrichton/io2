fn main() {
    for i in "..".utf16_units() {
        println!("{:?}", i as u8 as char);
    }
}
