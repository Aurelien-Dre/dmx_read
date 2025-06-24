# DMX reader

## My project
My project is to read a DMX frame using the embassy crate and a the USART module of my STM32F072RB

## Bug
Currently, I try to run my programm with ``DEFMT_LOG=warn, cargo run --release`` to read some UART frames but the buffer init cut my process and lose some datas 



