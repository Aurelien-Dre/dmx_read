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
    let mut tx_buffer = [0u8; 513];
    let mut rx_buffer = [0u8; 1000];

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
    let mut buffer_tx_first: [u8; 1] = [1; 1]; //Buffer to store the first frame
    let buffer_tx_second: [u8; 2] = [2; 2]; //Buffer to store a second frame
    let mut buffer_rx: [u8; 513] = [0; 513]; //Buffer to store the result frame
                                             //------------Variables-----------

    loop {
        let _ = serial.read(&mut buffer_tx_first).await;
        let _ = serial.read_exact(&mut buffer_rx).await;
        let _ = serial.write(&buffer_rx).await;
    }
}
