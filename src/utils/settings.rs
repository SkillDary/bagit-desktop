use gtk::Settings;

#[derive(Debug)]
pub struct BagitSettings {
    pub settings: gtk::Settings,
}

impl Default for BagitSettings {
    fn default() -> Self {
        BagitSettings {
            settings: Settings::builder().build(),
        }
    }
}
