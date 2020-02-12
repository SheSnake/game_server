
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

    pub fn is_bai_bang(card: u8) -> bool {
        card / 4 == 27
    }

    pub fn begin_card(card: u8) -> bool {
        let ix = card / 4;
        ix == 0 || ix == 9 || ix == 18
    }

    pub fn end_card(card: u8) -> bool {
        let ix = card / 4;
        ix == 8 || ix == 17 || ix == 26
    }
}
