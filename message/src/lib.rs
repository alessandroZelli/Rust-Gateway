//use hex_literal::*;
extern crate hex;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::any::Any;
use std::collections::*;
use std::ops::Deref;
use std::str::*;
use typetag::*;

pub struct MessageBox {
    pub message_type: String,
    pub content: Box<dyn Message>,
}

impl MessageBox {
    pub fn get_content<T>(&self) -> Result<&dyn Message, &'static str>
    where
        T: Message + ?Sized,
    {
        let msg_type = self.message_type.as_str();
        match msg_type {
            "discovery" => {
                let self_ref: &dyn Message = &*self.content;
                Ok(self_ref)
            }
            "configuration" => {
                let self_ref: &dyn Message = &*self.content;
                Ok(self_ref)
            }
            "command" => {
                let self_ref: &dyn Message = &*self.content;
                Ok(self_ref)
            }
            _ => Err("message type field undeclared"),
        }
    }

    pub fn new<T>(message_type: &str, msg: T) -> Self
    where
        T: Message + 'static,
    {
        MessageBox {
            message_type: String::from(message_type),
            content: Box::new(msg),
        }
    }
    pub fn print_content(&self) {
        self.content.print();
    }
}
#[typetag::serde(tag = "type")]
pub trait Message {
    fn print(&self);
}

pub trait SerialMessage {
    fn to_serial_data(&self) -> Vec<u8>;
}

#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct DiscoveryMsg {
    pub command: String,
    pub switch: bool,
}

#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct ConfigMsg {
    pub target: String,
    pub address: String,
    pub device_id: String,
    pub data: String,
}

#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct CommandMsg {
    pub address: String,
    pub duration: String,
    pub command: String,
    pub device_id: String,
    pub data: Vec<String>,
}
#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct DTO {
    pub command: String,
    pub data: Vec<String>,
}
#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct McuDeclareMsg {
    pub address: u8,
    pub length: u8,
    pub data: Vec<(u8, u8)>,
}
#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct McuCollectMsg {
    pub address: u8,
    pub length: u8,
    pub data: HashMap<u8, (u8, u8)>,
}

impl McuCollectMsg {
    pub fn build(data: Vec<u8>) -> Self {
        let mut message = McuCollectMsg {
            address: 0,
            length: 0,
            data: HashMap::new(),
        };
        message.address = data[0];
        message.length = data[1];
        for (j, k, l) in data[2..].iter().tuples() {
            message.data.insert(*j, (*k, *l));
        }
        message
    }

    pub fn test_serde() {
        let data: Vec<u8> = vec![1, 3, 1, 2, 3];
        let message = McuCollectMsg::build(data);
        println!("{:#?}", message);
        let msg = serde_json::to_string(&message).unwrap();
        println!("{}", msg);
    }
}

impl McuDeclareMsg {
    pub fn build(data: Vec<u8>) -> Self {
        let mut message = McuDeclareMsg {
            address: 0,
            length: 0,
            data: vec![],
        };
        message.address = data[0];
        message.length = data[1];
        for (i, j) in data[2..].iter().tuples() {
            message.data.push((*i, *j));
        }
        message
    }

    pub fn test_serde() {
        let data: Vec<u8> = vec![1, 3, 1, 2, 3];
        let message = McuDeclareMsg::build(data);
        println!("{:#?}", message);
        let msg = serde_json::to_string(&message).unwrap();
        println!("{}", msg);
    }
}
#[test]
fn collect_builder() {
    let data: Vec<u8> = vec![1, 3, 1, 2, 3];
    let message = McuCollectMsg::build(data);
    let test_var = (2u8, 3u8);
    let tuple = message.data.get(&1u8).unwrap();
    assert_eq!(*tuple, test_var);
}

#[test]
fn pairing_builder() {
    let data: Vec<u8> = vec![1, 4, 2, 1, 3, 2];
    let message = McuDeclareMsg::build(data);
    let test_var = (3u8, 2u8);
    let tuple = message.data[1];
    assert_eq!(tuple, test_var);
}

impl DiscoveryMsg {
    pub fn build(req_body: &str) -> Result<DiscoveryMsg, &'static str> {
        let data_transfer: DTO = serde_json::from_str(req_body).unwrap();
        let mut msg = DiscoveryMsg::default();
        match data_transfer.data[0].to_lowercase().as_str() {
            "on" => {
                msg.switch = true;
                Ok(msg)
            }
            "off" => {
                msg.switch = false;
                Ok(msg)
            }
            _ => Err("unexpected Parameter"),
        }
    }
}
#[typetag::serde]
impl Message for DiscoveryMsg {
    fn print(&self) {
        match self.switch {
            true => println!("On"),
            false => println!("Off"),
            _ => println!("Error"),
        }
    }
}

impl ConfigMsg {
    pub fn build(topic_vector: Vec<&str>, req_body: &str) -> Result<ConfigMsg, &'static str> {
        let mut msg = ConfigMsg::default();
        match topic_vector[2] {
            "cron" => {
                msg.target = String::from("cron");
                msg.address = topic_vector[3].to_string();
                msg.device_id = topic_vector[4].to_string();
                msg.data = String::from(req_body);
                Ok(msg)
            }
            _ => Err("Unexpected value in Config message body"),
        }
    }
}

#[typetag::serde]
impl Message for ConfigMsg {
    fn print(&self) {
        let stringified = format!(
            "{} - {} - {} - {}",
            self.target, self.address, self.device_id, self.data
        );
        println!("{}", stringified);
    }
}

impl CommandMsg {
    pub fn build(topic_vector: Vec<&str>, req_body: &str) -> Result<CommandMsg, &'static str> {
        let mut msg = CommandMsg::default();
        msg.address = topic_vector[2].parse().unwrap();
        msg.device_id = topic_vector[3].parse().unwrap();
        let data_transfer: DTO = serde_json::from_str(req_body).unwrap();
        msg.command = data_transfer.command;
        let msg_length = data_transfer.data.len() + 2;
        msg.data = data_transfer.data;
        msg.duration = msg_length.to_string();
        Ok(msg)
    }
}

impl SerialMessage for CommandMsg {
    fn to_serial_data(&self) -> Vec<u8> {
        let mut serial_data = vec![];
        let byte_addr: u8 = hexconverter::hexstring_to_u8(&self.address);
        let byte_command: u8 = hexconverter::hexstring_to_u8(&self.command);
        let byte_duration: u8 = self.duration.parse().unwrap();
        let byte_device_id: u8 = hexconverter::hexstring_to_u8(&self.device_id);
        serial_data.push(byte_addr);
        serial_data.push(byte_duration);
        serial_data.push(byte_command);
        serial_data.push(byte_device_id);
        for i in &self.data {
            let j: u8 = hexconverter::hexstring_to_u8(i);
            serial_data.push(j);
        }
        serial_data
    }
}
#[typetag::serde]
impl Message for CommandMsg {
    fn print(&self) {
        let stringified = format!(
            "{} - {} - {} - {:?}",
            self.address, self.duration, self.command, self.data
        );
        println!("{}", stringified);
    }
}

mod hexconverter {
    use std::collections::*;

    pub fn hexstring_to_u8(str_num: &str) -> u8 {
        let mut hex_map = HashMap::new();
        let value_array: Vec<char> = vec![
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
        ];
        let mut i = 0;
        for v in value_array {
            hex_map.insert(v, i);
            i += 1;
        }

        let str_num = str_num.to_string().to_lowercase();
        let str_num_array: Vec<&str> = str_num.split('x').collect();
        let num = str_num_array[1];
        let mut result: u8 = 0;
        let mut counter = 0;
        for i in num.to_string().chars().rev() {
            if counter == 0 {
                for (key, value) in &hex_map {
                    if *key == i {
                        result += value;
                    }
                }
            } else {
                for (key, value) in &hex_map {
                    if *key == i {
                        let to_add = value << 4;
                        result += to_add;
                    }
                }
            }
            counter += 1;
        }
        result
    }

    #[test]
    fn hex_converter() {
        let hex_string = "0xFF";
        let num = hexstring_to_u8(hex_string);
        assert_eq!(num, 255)
    }
}
