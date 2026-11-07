use agentla_bridges::i2c_tinyusb::*;
use embedded_hal::i2c::{Error, ErrorKind, ErrorType, I2c, NoAcknowledgeSource};

#[derive(Debug)]
struct MockI2cError;

impl Error for MockI2cError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
    }
}

struct MockI2c {
    ack_addr: u8,
    written_data: Vec<u8>,
    read_payload: Vec<u8>,
}

impl MockI2c {
    fn new(ack_addr: u8) -> Self {
        Self {
            ack_addr,
            written_data: Vec::new(),
            read_payload: Vec::new(),
        }
    }
}

impl ErrorType for MockI2c {
    type Error = MockI2cError;
}

impl I2c for MockI2c {
    fn read(&mut self, address: u8, read: &mut [u8]) -> Result<(), Self::Error> {
        if address != self.ack_addr {
            return Err(MockI2cError);
        }
        let len = read.len().min(self.read_payload.len());
        read[..len].copy_from_slice(&self.read_payload[..len]);
        Ok(())
    }

    fn write(&mut self, address: u8, write: &[u8]) -> Result<(), Self::Error> {
        if address != self.ack_addr {
            return Err(MockI2cError);
        }
        self.written_data.extend_from_slice(write);
        Ok(())
    }

    fn write_read(&mut self, address: u8, write: &[u8], read: &mut [u8]) -> Result<(), Self::Error> {
        self.write(address, write)?;
        self.read(address, read)?;
        Ok(())
    }

    fn transaction(
        &mut self,
        _address: u8,
        _operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_echo_command() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x48);
    let mut buf = [0u8; 4];

    let n = handler
        .handle_control_request(&mut mock, CMD_ECHO, 0x1234, 0, &mut buf)
        .expect("echo should succeed");
    assert_eq!(n, 2);
    assert_eq!(&buf[..2], &0x1234u16.to_le_bytes());
}

#[test]
fn test_get_func_command() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x48);
    let mut buf = [0u8; 4];

    let n = handler
        .handle_control_request(&mut mock, CMD_GET_FUNC, 0, 0, &mut buf)
        .expect("get_func should succeed");
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], &I2C_FUNC_ALL.to_le_bytes());
}

#[test]
fn test_set_delay_and_get_status() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x48);
    let mut buf = [0u8; 4];

    handler
        .handle_control_request(&mut mock, CMD_SET_DELAY, 100, 0, &mut buf)
        .expect("set_delay should succeed");
    assert_eq!(handler.delay_us(), 100);

    let n = handler
        .handle_control_request(&mut mock, CMD_GET_STATUS, 0, 0, &mut buf)
        .expect("get_status should succeed");
    assert_eq!(n, 1);
    assert_eq!(buf[0], STATUS_IDLE);
}

#[test]
fn test_i2c_io_read_success() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x48);
    mock.read_payload = vec![0xDE, 0xAD, 0xBE, 0xEF];

    let mut buf = [0u8; 4];
    let req = CMD_I2C_IO | CMD_I2C_IO_BEGIN | CMD_I2C_IO_END | CMD_I2C_IO_READ;
    let n = handler
        .handle_control_request(&mut mock, req, 0, 0x48, &mut buf)
        .expect("read should succeed");

    assert_eq!(n, 4);
    assert_eq!(buf, [0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(handler.last_status(), STATUS_ADDRESS_ACK);
}

#[test]
fn test_i2c_io_read_nak() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x48);

    let mut buf = [0u8; 4];
    let req = CMD_I2C_IO | CMD_I2C_IO_BEGIN | CMD_I2C_IO_END | CMD_I2C_IO_READ;
    let err = handler
        .handle_control_request(&mut mock, req, 0, 0x50, &mut buf)
        .expect_err("address 0x50 should NAK");

    assert!(matches!(err, I2cTinyUsbError::I2c(_)));
    assert_eq!(handler.last_status(), STATUS_ADDRESS_NAK);
}

#[test]
fn test_i2c_io_write_success() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x50);

    let mut data = [0x01, 0x02, 0x03];
    let req = CMD_I2C_IO | CMD_I2C_IO_BEGIN | CMD_I2C_IO_END;
    let n = handler
        .handle_control_request(&mut mock, req, 0, 0x50, &mut data)
        .expect("write should succeed");

    assert_eq!(n, 3);
    assert_eq!(mock.written_data, vec![0x01, 0x02, 0x03]);
    assert_eq!(handler.last_status(), STATUS_ADDRESS_ACK);
}

#[test]
fn test_control_in_and_out_methods() {
    let mut handler = I2cTinyUsbHandler::new();
    let mut mock = MockI2c::new(0x20);
    mock.read_payload = vec![0x42];

    // Out write
    handler
        .handle_control_out(&mut mock, CMD_I2C_IO, 0, 0x20, &[0xAA])
        .expect("control out write");
    assert_eq!(mock.written_data, vec![0xAA]);
    assert_eq!(handler.last_status(), STATUS_ADDRESS_ACK);

    // In read
    let mut in_buf = [0u8; 1];
    let len = handler
        .handle_control_in(&mut mock, CMD_I2C_IO, 0, 0x20, &mut in_buf)
        .expect("control in read");
    assert_eq!(len, 1);
    assert_eq!(in_buf[0], 0x42);
    assert_eq!(handler.last_status(), STATUS_ADDRESS_ACK);
}
