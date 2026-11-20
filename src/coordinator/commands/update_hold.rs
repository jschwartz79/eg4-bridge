use crate::prelude::*;
use crate::eg4::inverter::WaitForReply;
use crate::eg4::{
    packet::{Packet, RegisterBit, DeviceFunction, TranslatedData},
    inverter::ChannelData,
};

pub struct UpdateHold {
    channels: Channels,
    inverter: config::Inverter,
    register: u16,
    mask: u16,
    value: u16,
}

impl UpdateHold {
    pub fn new(
        channels: Channels,
        inverter: config::Inverter,
        register: u16,
        bit: RegisterBit,
        enable: bool,
    ) -> Self {
        let mask = bit as u16;
        let value = if enable { mask } else { 0 };
        Self {
            channels,
            inverter,
            register,
            mask,
            value,
        }
    }

    pub fn new_bitfield(
        channels: Channels,
        inverter: config::Inverter,
        register: u16,
        mask: u16,
        shifted_value: u16,
    ) -> Self {
        Self {
            channels,
            inverter,
            register,
            mask,
            value: shifted_value & mask,
        }
    }

    pub async fn run(&self) -> Result<()> {
        if self.inverter.read_only() {
            bail!(
                "Cannot update hold register {} - inverter {} is in read-only mode",
                self.register,
                self.inverter.datalog().map(|s| s.to_string()).unwrap_or_default()
            );
        }

        let mut receiver = self.channels.from_inverter.subscribe();

        // First read the current value
        let read_packet = Packet::TranslatedData(TranslatedData {
            datalog: self.inverter.datalog().expect("datalog must be set"),
            device_function: DeviceFunction::ReadHold,
            inverter: self.inverter.serial().expect("serial must be set"),
            register: self.register,
            values: vec![1, 0],
        });

        self.channels
            .to_inverter
            .send(ChannelData::Packet(read_packet.clone()))
            .map_err(|e| anyhow!("send(to_inverter) failed: {}", e))?;

        let read_packet = receiver.wait_for_reply(&read_packet).await?;
        let current_value = read_packet.value();
        let new_value = (current_value & !self.mask) | (self.value & self.mask);

        // Now write the new value
        let write_packet = Packet::TranslatedData(TranslatedData {
            datalog: self.inverter.datalog().expect("datalog must be set"),
            device_function: DeviceFunction::WriteSingle,
            inverter: self.inverter.serial().expect("serial must be set"),
            register: self.register,
            values: new_value.to_le_bytes().to_vec(),
        });

        self.channels
            .to_inverter
            .send(ChannelData::Packet(write_packet.clone()))
            .map_err(|e| anyhow!("send(to_inverter) failed: {}", e))?;

        let write_packet = receiver.wait_for_reply(&write_packet).await?;
        if write_packet.value() != new_value {
            bail!(
                "failed to update register {:?}, got back value {} (wanted {})",
                self.register,
                write_packet.value(),
                new_value
            );
        }

        Ok(())
    }
}
