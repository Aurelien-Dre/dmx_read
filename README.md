# DMX reader

## My project
My project is to read a DMX frame using the embassy crate and a the USART module of my STM32F072RB

## Bug
Currently, the reception is interrupted during DMX frame transmission, so I hear your proposals for improving my code.
I also limit the logging level by running the code with ``DEFMT_LOG=warn, cargo run --release``



