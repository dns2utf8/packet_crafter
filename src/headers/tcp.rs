use crate::{AsBeBytes, finalize_checksum, ip_sum};
use super::{Header, PacketData, Protocol};

struct PseudoHeader {
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    data_len: u16,
}

#[derive(AddGetter, AddSetter)]
pub struct TcpHeader {
    #[get]
    #[set]
    src_port: u16,
    #[get]
    #[set]
    dst_port: u16,
    #[get]
    flags: u8,
    #[set]
    window: u16,
    pseudo_header: Option<PseudoHeader>,
}

pub enum TcpFlags {
    Urg,
    Ack,
    Psh,
    Rst,
    Syn,
    Fin,
}

impl TcpHeader {
    pub fn new(src_port: u16, dst_port: u16) -> Self {
        TcpHeader {
            src_port: src_port,
            dst_port: dst_port,
            window: 0xffff,
            flags: 0,
            pseudo_header: None,
        }
    }

    pub fn set_flag(&mut self, f: TcpFlags) {
        match f {
            TcpFlags::Urg => self.flags = self.flags | 0b00100000,
            TcpFlags::Ack => self.flags = self.flags | 0b00010000,
            TcpFlags::Psh => self.flags = self.flags | 0b00001000,
            TcpFlags::Rst => self.flags = self.flags | 0b00000100,
            TcpFlags::Syn => self.flags = self.flags | 0b00000010,
            TcpFlags::Fin => self.flags = self.flags | 0b00000001,
        }
    }

    pub fn set_pseudo_header(&mut self, src_ip: [u8; 4], dst_ip: [u8; 4], packet_data: &[u8]) {
        let len = packet_data.len();
        if len > (0xffff - 20) as usize {
            panic!("too much data");
        }
        self.pseudo_header = Some(PseudoHeader {
            src_ip,
            dst_ip,
            data_len: 20 + (len as u16),
        });
    }

    pub fn parse(raw_data: &[u8]) -> Self {
        if raw_data.len() < 20 {
            panic!("Parse TCP header: invalid length");
        }
        Self {
            src_port: ((raw_data[0] as u16) << 8) + raw_data[1] as u16,
            dst_port: ((raw_data[2] as u16) << 8) + raw_data[3] as u16,
            flags: raw_data[13],
            window: ((raw_data[14] as u16) << 8) + raw_data[15] as u16,
            pseudo_header: None,
        }
    }
}

impl Header for TcpHeader {
    fn make(self) -> PacketData {
        let src_p = self.src_port.split_to_bytes();
        let dst_p = self.dst_port.split_to_bytes();
        let window_bytes = self.window.split_to_bytes();
        let mut packet = vec![
            src_p[0],
            src_p[1],
            dst_p[0],
            dst_p[1],
            0,
            0,
            0,
            0, // Seq num
            0,
            0,
            0,
            0, // Ack num
            0, // Offset + 4 of the reserved bits, the other 2 of the 6 total reserved bits are included at the start of the `flags` byte
            self.flags,
            window_bytes[0],
            window_bytes[1],
            0,
            0,
            0,
            0, // Urgent Pointer -> Should do this at some point
        ];

        // calculate checksum
        if let None = self.pseudo_header {
            panic!("Please set the pseudo header data before calculating the checksum");
        }
        let mut val = 0u32;
        val += ip_sum(self.pseudo_header.as_ref().unwrap().src_ip);
        val += ip_sum(self.pseudo_header.as_ref().unwrap().dst_ip);
        val += 6; // this covers the reserved byte, plus the protocol field, which we set to 6 since that is the value for TCP
        val += 20; // header length (in bytes) : when there are no options+padding present, the header length is 20 bytes
        val += self.pseudo_header.as_ref().unwrap().data_len as u32;
        let checksum = finalize_checksum(val).split_to_bytes();

        packet[16] = checksum[0];
        packet[17] = checksum[1];
        packet
    }

    fn get_proto(&self) -> Protocol {
        Protocol::TCP
    }

    fn get_length(&self) -> u8 {
        20
    }

    fn get_min_length() -> u8 {
        20
    }
}
