#![no_std]
#![no_main]

mod fmt;

#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use defmt::info;
//------------Imports-----------
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    peripherals::{self},
    usart::{self, BufferedUart, Config, StopBits},
};
use embassy_time::Timer;
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
    let mut tx_buffer = [0u8; 50];
    let mut rx_buffer = [0u8; 15];

    let mut config = Config::default();
    config.baudrate = 250000;
    config.stop_bits = StopBits::STOP2;
    let mut serial = BufferedUart::new_with_de(
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
    let buffer_tx_first: [u8; 10] = [1; 10]; //Buffer to store DMX512 frame
    let buffer_tx_second: [u8; 10] = [2; 10]; //Buffer to store DMX512 frame
    let mut buffer_rx: [u8; 10] = [0; 10]; //Buffer to store DMX512 frame
                                           //------------Variables-----------

    //first read cycle
    let _ = serial.write(&buffer_tx_first).await;
    let a = serial.read(&mut buffer_rx).await; //fill the 10 first slots of the DMA

    //second read cycle
    let _ = serial.write(&buffer_tx_second).await;
    let b = serial.read(&mut buffer_rx).await; //is blocked by the maximal size of the dma and only return 5 numbrers form the second buffer

    //print the buffer_rx for debug
    let _ = serial.write(&buffer_rx).await;
    info!("{} and {} bits have been readen", a.unwrap(), b.unwrap());
    loop {}
}
