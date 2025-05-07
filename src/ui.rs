use slint::{ModelRc, PlatformError, SharedString, VecModel};

slint::include_modules!();

pub struct UI {
    pub ui: AppWindow,
}

impl UI {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            ui: AppWindow::new()?,
        })
    }

    pub fn run(&mut self) -> Result<(), PlatformError> {
        self.ui.run()
    }

    pub fn set_device_lists(&mut self, input_devices: Vec<String>, output_devices: Vec<String>) {
        let input_device_model = self.get_model_from_device_list(&input_devices);
        self.ui.set_input_device_list(input_device_model);

        let output_device_model = self.get_model_from_device_list(&output_devices);
        self.ui.set_output_device_list(output_device_model);
    }

    pub fn set_channel_lists(&mut self, input_channels: Vec<String>, output_channels: Vec<String>) {
        let input_channel_model = self.get_model_from_channel_list(&input_channels);
        self.ui.set_input_channel_list(input_channel_model);

        let output_channel_model = self.get_model_from_channel_list(&output_channels);
        self.ui.set_output_channel_list(output_channel_model);
    }

    pub fn set_default_devices(&mut self, input_device: String, output_device: String) {
        self.ui
            .set_current_input_device(SharedString::from(input_device));
        self.ui
            .set_current_output_device(SharedString::from(output_device));
    }

    pub fn set_default_channels(
        &mut self,
        left_input_channel: String,
        right_input_channel: String,
        left_output_channel: String,
        right_output_channel: String,
    ) {
        self.ui
            .set_left_current_input_channel(SharedString::from(left_input_channel));
        self.ui
            .set_right_current_input_channel(SharedString::from(right_input_channel));
        self.ui
            .set_left_current_output_channel(SharedString::from(left_output_channel));
        self.ui
            .set_right_current_output_channel(SharedString::from(right_output_channel));
    }

    pub fn on_selected_input_device(&mut self, callback: impl Fn(String) + 'static) {
        self.ui
            .on_selected_input_device(move |device: SharedString| {
                callback(device.to_string());
            });
    }

    pub fn on_selected_output_device(&mut self, callback: impl Fn(String) + 'static) {
        self.ui
            .on_selected_output_device(move |device: SharedString| {
                callback(device.to_string());
            });
    }

    pub fn on_left_selected_input_channel(&mut self, callback: impl Fn(String) + 'static) {
        self.ui
            .on_left_selected_input_channel(move |channel: SharedString| {
                callback(channel.to_string());
            });
    }

    pub fn on_left_selected_output_channel(&mut self, callback: impl Fn(String) + 'static) {
        self.ui
            .on_left_selected_output_channel(move |channel: SharedString| {
                callback(channel.to_string());
            });
    }

    pub fn on_right_selected_input_channel(&mut self, callback: impl Fn(String) + 'static) {
        self.ui
            .on_right_selected_input_channel(move |channel: SharedString| {
                callback(channel.to_string());
            });
    }

    pub fn on_right_selected_output_channel(&mut self, callback: impl Fn(String) + 'static) {
        self.ui
            .on_right_selected_output_channel(move |channel: SharedString| {
                callback(channel.to_string());
            });
    }

    fn get_model_from_device_list(&mut self, devices: &[String]) -> ModelRc<SharedString> {
        let name_list: Vec<SharedString> = devices.iter().map(SharedString::from).collect();
        ModelRc::new(VecModel::from_slice(name_list.as_slice()))
    }

    fn get_model_from_channel_list(&mut self, channels: &[String]) -> ModelRc<SharedString> {
        let channel_list: Vec<SharedString> = channels.iter().map(SharedString::from).collect();
        ModelRc::new(VecModel::from_slice(channel_list.as_slice()))
    }
}
