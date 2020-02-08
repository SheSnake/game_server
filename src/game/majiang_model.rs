
pub enum Majiang {
    Wan(u8),
    Tiao(u8),
    Bin(u8),
    BaiBan,
}

impl Majiang {
    pub fn format(card: &Majiang) -> String {
        match card {
            Majiang::Wan(i) => format!("{}-W", i),
            Majiang::Tiao(i) => format!("{}-T", i),
            Majiang::Bin(i) => format!("{}-B", i),
            Majiang::BaiBan => format!("Bai"),
        }
    }
}
