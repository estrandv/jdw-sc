use std::thread;
use zmq::Socket;

// TODO: Subscriber should be a git dependency instead, see comments in jdw-keyboard
pub struct ZMQSubscriber {
    socket: Socket
}

impl ZMQSubscriber {
    pub fn new() -> ZMQSubscriber {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::SUB).unwrap();
        socket.connect("tcp://localhost:5560").unwrap();
        socket.set_subscribe("JDW.SEQ.BPM".as_bytes());
        socket.set_subscribe("JDW.SEQ.QUEUE".as_bytes());
        socket.set_subscribe("JDW.NSET.NOTE".as_bytes());
        socket.set_subscribe("JDW.ADD.NOTE".as_bytes());

        ZMQSubscriber {
            socket
        }
    }

    pub fn recv(&self) -> ZMQMsg {
        let msg = self.socket.recv_msg(0).unwrap();

        let decoded_msg = msg.as_str().unwrap().split("::").collect::<Vec<&str>>();

        let msg_type = decoded_msg.get(0).unwrap().to_string();

        let msg_timestamp = decoded_msg.get(1).unwrap_or(&"").to_string();
        let type_handle = format!("{}::{}::", msg_type, msg_timestamp);

        let json_msg = msg.as_str()
            .unwrap()
            .split(&type_handle)
            .collect::<Vec<&str>>()
            .get(1).unwrap_or(&"").to_string();

        println!("message: {}, json: {}", &type_handle, &json_msg);

        ZMQMsg {
            msg_type,
            timestamp: msg_timestamp,
            json_contents: json_msg
        }
    }
}

pub struct ZMQMsg {
    pub msg_type: String,
    pub timestamp: String,
    pub json_contents: String
}