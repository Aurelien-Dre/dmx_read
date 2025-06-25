# DMX reader

## My project
My project is to read a DMX frame using the embassy crate and a the USART module of my STM32F072RB

## Bug
Currently, I try to run my programm with ``DEFMT_LOG=warn, cargo run --release`` to read some UART frames. 
I send a frame of teen 1 and a frame of teen 2 with my tx pin wired on my rx pin. 
I read on each write call but I reach the end of the DMA buffer and the final result is not a local buffer full of 2 but with half of 1.
So my buffer drop some data when reaching the end of it internal buffer like you can see on this graphic :

![Logical graphic of the STM32 behaviour](https://github.com/Aurelien-Dre/dmx_read/blob/master/my_result.png?raw=true)

I throught that after the first call to the read function, my virtual DMA buffer would be release but that's not what happen and only the half part of my message was collected.

Do you know a way to release the buffer or to fill it in order to correcly read the next frame ?

