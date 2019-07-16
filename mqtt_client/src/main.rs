use amiquip::Result as amiquipResult;
use amiquip::{
    Connection, ConsumerMessage, ConsumerOptions, Exchange, Publish, QueueDeclareOptions,
};
use message::*;
use rumqtt::{
    client::Notification, MqttClient, MqttOptions, QoS, ReconnectOptions, SecurityOptions,
};

use serde_json;
use std::str;
use std::sync::mpsc;
use std::thread;
mod serial_module;
use std::io;
use std::io::Write;
use std::mem;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    /*  let security_options =
    SecurityOptions::UsernamePassword("gateway".to_string(), "gateway".to_string()); */
    let reconnect_options = ReconnectOptions::Always(5);
    let mqtt_options = MqttOptions::new("mqttrust", "localhost", 1883)
        .set_keep_alive(60)
        .set_reconnect_opts(reconnect_options);
    // .set_security_opts(security_options);
    let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options).unwrap(); //  creates publisher and receiver

    mqtt_client.subscribe("home/#", QoS::AtLeastOnce).unwrap();

    // MQTT TO AMQP
    let (tx, rx) = mpsc::channel();
    // AMQP PARSING
    let (tx1, rx1) = mpsc::channel();
    // GATEWAY MESSAGES
    let (tx2, rx2) = mpsc::channel(); // Config
                                      // SERIAL COMMANDS
    let (tx3, rx3) = mpsc::channel(); //  Serial Load Balancer
    let (tx4, rx4) = mpsc::channel(); //  Command & Discovery
    let (tx5, rx5) = mpsc::channel(); //  User Activated
                                      //let (tx6, rx6) = mpsc::channel(); //  Automatic Collect

    let handle_receive = thread::spawn(move || {
        println!("handle receive spawned");
        for notification in notifications.iter() {
            match notification {
                Notification::Publish(body) => {
                    //  FIXARE ERRORE SU MESSAGGIO VUOTO
                    let topic = &body.topic_name;
                    let payload = str::from_utf8(&body.payload).unwrap();
                    let msg_box = parse_mqtt_message(topic, payload);

                    match msg_box {
                        Ok(t) => {
                            let msg_type = t.message_type;
                            match msg_type.as_str() {
                                "discovery" => {
                                    let json_discovery =
                                        serde_json::to_string(&*t.content).unwrap();

                                    tx.send(json_discovery).unwrap();
                                }
                                "configuration" => {
                                    let json_configuration =
                                        serde_json::to_string(&*t.content).unwrap();

                                    tx.send(json_configuration).unwrap();
                                }
                                "command" => {
                                    let json_command = serde_json::to_string(&*t.content).unwrap();

                                    tx.send(json_command).unwrap();
                                }
                                &_ => {}
                            }
                        }
                        Err(e) => {
                            println!("{:#?}", e);
                        }
                    }
                }
                _ => (),
            }
        }
    });

    let amqp_sender = thread::spawn(move || {
        println!("amqp_sender spawned");
        for mqtt_received in rx {
            //println!("GOT: {}", mqtt_received);
            // AMQP CLIENT SENDING TO RABBIT
            send_amqp(mqtt_received.as_str()).expect("unable to send");
        }
    });

    let amqp_consumer = thread::spawn(move || {
        println!("amqp_consumer spawned");
        consume_amqp(tx1, "rust_gateway").unwrap();
    });

    let amqp_parser = thread::spawn(move || {
        println!("json parser spawned");
        for rabbit_received in rx1 {
            let json_msg = rabbit_received.as_str();
            let msg_box = parse_amqp_string(json_msg).unwrap();

            match msg_box.message_type.to_lowercase().as_str() {
                "configuration" => {
                    let string_msg = serde_json::to_string(&*msg_box.content).unwrap();
                    /*   println!("{}",string_msg); */
                    tx2.send(string_msg).unwrap();
                }
                "command" => {
                    let string_msg = serde_json::to_string(&*msg_box.content).unwrap();
                    /*  println!("{}",string_msg); */
                    tx3.send(string_msg).unwrap();
                }
                "discovery" => {
                    let string_msg = serde_json::to_string(&*msg_box.content).unwrap();
                    /*   println!("{}",string_msg); */
                    tx3.send(string_msg).unwrap();
                }
                _ => (),
            };
        }
    });

    let configurator = thread::spawn(move || {
        println!("configurator spawned");
        for i in rx2 {
            println!("{:#?}", i);
        }
    });

    let serial_balancer = thread::spawn(move || {
        for i in rx3 {
            if i.contains("DiscoveryMsg") {
                println!("{:#?}", i);
                tx4.send(i).unwrap();
            } else {
                let msg: CommandMsg = serde_json::from_str(&i).unwrap();
                match msg.command.to_lowercase().as_str() {
                    "0x01" => {
                        let dto = serde_json::to_string(&msg).unwrap();
                        println!("{}", dto);
                        tx5.send(dto).unwrap();
                    }
                    _ => {
                        let dto = serde_json::to_string(&msg).unwrap();
                        println!("{}", dto);
                        tx4.send(dto).unwrap();
                    }
                }
            }
        }
    });
    let port_name = serial_module::get_portname();
    let mut serial_port = serial_module::init_serial_port(port_name);
    let mut serial_read = serial_port.try_clone().expect("failed to clone the port");
    /* let address_vector: Vec<u8> = vec![];
    let address_vector_mutex = Arc::new(Mutex::new(address_vector)); */
    let mut address_count = Arc::new(Mutex::new(0));
    let serial_io = thread::spawn(move || loop {
        match rx4.try_recv() {
            Ok(msg_string) => {
                println!("hello from Ok arm: {:#?}", msg_string);
                if msg_string.contains("DiscoveryMsg") {
                    let message: DiscoveryMsg = serde_json::from_str(&msg_string).unwrap();
                    match message.switch {
                        true => {
                            let mut buffer: [u8; 1] = unsafe { mem::uninitialized() };
                            println!("DISCOVERY MODE ON!");
                            let mut response_vector: Vec<u8> = vec![];
                            let mut reading_discovery = true;
                            let discovery_signal: Vec<u8> = vec![255]; 
                            serial_port.write(&discovery_signal); 
                            match serial_read.read(&mut buffer) {
                                Ok(bytes) => {
                                    if bytes == 1 {
                                        let received = buffer[0];
                                        if received == 255 {
                                            let mut address_count = address_count.lock().unwrap();
                                            *address_count+=1;
                                          /*   let address: Vec<u8> = vec![get_address(&*address_v)]; */
                                            let mut address: Vec<u8> = vec![address_count.clone()];
                                            serial_port.write(&address);
                                           
                                            
                                            while reading_discovery {
                                                match serial_read.read(&mut buffer) {
                                                    Ok(bytes) => {
                                                        if bytes == 1 {
                                                            response_vector.push(buffer[0]);
                                                        }
                                                    }
                                                    Err(ref e)
                                                        if e.kind() == io::ErrorKind::TimedOut =>
                                                    {
                                                        println!("{:#?}", response_vector);
                                                        reading_discovery = false;
                                                    }
                                                    Err(e) => {
                                                        eprintln!("{:?}", e);
                                                        reading_discovery = false;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                                    println!("{:#?}", response_vector);
                                    reading_discovery = false;
                                    // TRANSMIT
                                }
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        false => {
                            println!("DISCOVERY MODE OFF!");
                            break;
                        }
                    }
                } else {
                    let mut buffer: [u8; 1] = unsafe { mem::uninitialized() };
                    let message: CommandMsg = serde_json::from_str(&msg_string).unwrap();
                    let data = message.to_serial_data();
                    serial_port.write(&data).unwrap();
                    //println!("{:#?}", &data);
                    let mut response_vector: Vec<u8> = vec![];
                    let mut reading = true;
                    while reading {
                        match serial_read.read(&mut buffer) {
                            Ok(bytes) => {
                                if bytes == 1 {
                                    println!("Received: {:?}", buffer);
                                    response_vector.push(buffer[0]);
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                                println!("{:#?}", response_vector);
                                reading = false;
                                // TRANSMIT
                            }
                            Err(e) => eprintln!("{:?}", e),
                            /* println!("{:#?}",buffer);
                            let data_msg: McuCollectMsg = McuCollectMsg::build(buffer);
                            println!("{:#?}",data_msg);
                            let data = serde_json::to_string(&data_msg).unwrap();
                            println!("{}", data);
                            tx6.send(data).unwrap() */
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
            Err(e) => match rx5.try_recv() {
                Ok(msg_string) => {
                    println!("hello from error arm: {:#?}", msg_string);
                    let mut buffer: [u8; 1] = unsafe { mem::uninitialized() };
                    let message: CommandMsg = serde_json::from_str(&msg_string).unwrap();
                    let data = message.to_serial_data();
                    //println!("{:#?}", data);
                    serial_port.write(&data).unwrap();
                    let mut response_vector: Vec<u8> = vec![];
                    let mut reading2 = true;
                    while reading2 {
                        match serial_read.read(&mut buffer) {
                            Ok(bytes) => {
                                if bytes == 1 {
                                    println!("Received: {:?}", buffer);
                                    response_vector.push(buffer[0]);
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                                println!("{:#?}", response_vector);
                                reading2 = false;
                                // TRANSMIT
                            }
                            Err(e) => eprintln!("{:?}", e),
                            /* println!("{:#?}",buffer);
                            let data_msg: McuCollectMsg = McuCollectMsg::build(buffer);
                            println!("{:#?}",data_msg);
                            let data = serde_json::to_string(&data_msg).unwrap();
                            println!("{}", data);
                            tx6.send(data).unwrap() */
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(_e) => (), /*  println!("{}", e), */
            },
        }
    });
    /*  for i in rx6 {
        println!("{:?}", i);
    } */

    let mut thread_pool = vec![];
    thread_pool.push(handle_receive);
    thread_pool.push(amqp_consumer);
    thread_pool.push(amqp_sender);
    thread_pool.push(amqp_parser);
    thread_pool.push(configurator);
    thread_pool.push(serial_balancer);
    //thread_pool.push(serial_io);
    for i in thread_pool {
        i.join().unwrap();
    }
    //handle_receive.join().unwrap();
}
pub fn get_address(address_vector: &[u8]) -> u8 {
    let count: u8 = address_vector.len() as u8;
    if count == 0 {
        1
    } else {
        let address: u8 = count + 1u8;
        address
    }
}
pub fn parse_mqtt_message(topic: &str, req_body: &str) -> Result<MessageBox, &'static str> {
    let topic_fields: Vec<&str> = topic.split("/").collect();
    let req_body = req_body.to_ascii_lowercase();
    if topic_fields.len() < 2 {
        return Err("message on undeclared topic");
    } else {
        match Some(topic_fields[1]) {
            Some("discovery") => {
                let msg = DiscoveryMsg::build(&req_body)?;
                let msgbox = MessageBox::new("discovery", msg);
                //msgbox.print_content();
                Ok(msgbox)
            }
            Some("config") => {
                let topic_vector = topic_fields.clone();
                let msg = ConfigMsg::build(topic_vector, &req_body)?;
                let msgbox = MessageBox::new("configuration", msg);
                //msgbox.print_content();
                Ok(msgbox)
            }
            Some("command") => {
                let topic_vector = topic_fields.clone();
                let msg = CommandMsg::build(topic_vector, &req_body)?;
                let msgbox = MessageBox::new("command", msg);
                //msgbox.print_content();
                Ok(msgbox)
            }
            None => Err("No topic found"),
            _ => Err("Unexpected topic in position 2"),
        }
    }
}

pub fn parse_amqp_string(json_message: &str) -> Result<MessageBox, &'static str> {
    println!("{}", json_message);
    if json_message.contains("DiscoveryMsg") {
        //println!("{}", json_message);
        let msg: DiscoveryMsg = serde_json::from_str(json_message).unwrap();
        //println!("{:#?}", msg);
        let msg_box = MessageBox::new("discovery", msg);
        //msg_box.print_content();
        Ok(msg_box)
    } else if json_message.contains("ConfigMsg") {
        //println!("{}", json_message);
        let msg: ConfigMsg = serde_json::from_str(json_message).unwrap();
        //println!("{:#?}", msg);
        let msg_box = MessageBox::new("config", msg);
        //msg_box.print_content();
        Ok(msg_box)
    } else if json_message.contains("CommandMsg") {
        //println!("{}", json_message);
        let msg: CommandMsg = serde_json::from_str(json_message).unwrap();
        //println!("{:#?}", msg);
        let msg_box = MessageBox::new("command", msg);
        //msg_box.print_content();
        Ok(msg_box)
    } else {
        Err("Unknown message type")
    }
}

pub fn send_amqp(message: &str) -> amiquipResult<()> {
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.open_channel(None)?;

    // Get a handle to the direct exchange on our channel.
    let exchange = Exchange::direct(&channel);

    // Publish a message to the "hello" queue.
    exchange.publish(Publish::new(message.as_bytes(), "rust_gateway"))?;
    //println!("Message sent: {}", message);
    connection.close()
}

pub fn consume_amqp(tx: mpsc::Sender<String>, queue_name: &str) -> amiquipResult<()> {
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;
    let channel = connection.open_channel(None)?;
    let queue = channel.queue_declare(queue_name, QueueDeclareOptions::default())?;
    let consumer = queue.consume(ConsumerOptions::default())?;

    println!("Waiting for messages. Press Ctrl-C to exit.");

    for (_i, message) in consumer.receiver().iter().enumerate() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                //println!("{}", body);
                tx.send(String::from(body)).expect("unable to send");
                consumer.ack(delivery)?;
            }
            other => {
                println!("Consumer ended: {:?}", other);
                break;
            }
        }
    }

    connection.close()
}
