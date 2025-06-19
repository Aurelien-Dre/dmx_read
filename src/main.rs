#![no_std]
#![no_main]

mod fmt;

#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

//------------Imports-----------
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    peripherals::{self},
    usart::{self, BufferedUart, Config, StopBits},
};
use embedded_io_async::{Read, Write};
//------------Imports-----------

//------------Interrupts-----------
bind_interrupts!(struct Irqs {
    USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
});
//------------Interrupts-----------

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    //------------USART initialisation-----------
    let mut config = Config::default();
    config.baudrate = 250000; //Baudrate for DMX512 reading
    config.stop_bits = StopBits::STOP2; //Number of stop bits for DMX512 reading

    let mut tx_buffer = [0u8; 513]; //Buffer for rx_DMA
    let mut rx_buffer = [0u8; 513]; //Buffer for tx_DMA

    let mut serial = BufferedUart::new_with_de(
        //Buffered Uart ready to bidirectionnal communication with DMX512 protocol
        p.USART1,
        Irqs,
        p.PA10,
        p.PA9,
        p.PA12,
        &mut tx_buffer,
        &mut rx_buffer,
        config,
    )
    .unwrap();
    //------------UART initialisation-----------

    //------------Variables-----------
    let mut buffer_dmx: [u8; 513] = [10; 513]; //Buffer to store DMX512 frame
                                               //------------Variables-----------

    loop {
        let _ = serial.read(&mut buffer_dmx).await; //Read until Mark Before Break (symbolised by a Framing error)
        let _ = serial.write(&buffer_dmx[1..2]).await; //Write a byte on the serial port to check on logical analyzed if the read was correctly done
    }
}
