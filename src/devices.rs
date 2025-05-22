use crate::errors::LocalError;

#[derive(Clone, Default, Debug)]
pub struct DeviceList {
    pub devices: Vec<String>,
    pub channels: Vec<Vec<String>>,
}

#[derive(Clone, Default, Debug)]
pub struct CurrentDevice {
    pub index: i32,
    pub name: String,
    pub left_channel: String,
    pub right_channel: Option<String>,
}

pub fn get_channel_indexes_from_channel_names(
    left_channel: &str,
    right_channel: &Option<String>,
) -> Result<(usize, Option<usize>), LocalError> {
    let left_index = get_index_from_name(left_channel)?;
    let mut right_index: Option<usize> = None;

    if right_channel.is_some() {
        right_index = Some(get_index_from_name(right_channel.as_ref().unwrap())?);
    }

    Ok((left_index, right_index))
}

fn get_index_from_name(channel: &str) -> Result<usize, LocalError> {
    let channel_number = channel
        .parse::<usize>()
        .map_err(|err| LocalError::ChannelIndex(err.to_string()))?;

    Ok(channel_number.saturating_sub(1))
}
