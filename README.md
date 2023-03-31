# esp-gallery :crab:
Wokwi-example with usage of my colegue's, Samuel Benko's, beautiful graphic examples (check [here](https://github.com/sambenko/esp32s3-box-examples)), but adapted for famous `ili9341` display)

## Instructions

```
cargo build --target riscv32imac-unknown-none-elf --release
```

### Execution with VSCode Wokwi extension  

Except of extension itself, you will need two files to execute the simulatuion inside of your VSCode IDE:
* wokwi.toml 
* diagram.json

Both of them are already provided in this repo and you can edit them as needed.

```
F1 -> Wokwi: Start simulation
```
In case you will have additional questions - feel free to open an issue :wink:



## Description
There's a short example for newcomers that shows, how 8x8 LED Matrix display is initialised on different Espressif boards (ESP32, ESP32S2, ESP32C3) in bare-metal, this config is for ESP32 in terms of example, you can find brief instructions for other chips below<br>


>### **P.S.** : Buttons on left are `up` and `down` correspondingly, button on top is `ok`. After image finished rendering press `ok` button to return back to main menu :wink:

## Tips for other chips
For `esp32` board target (and corresponding changes for paths in `wokwi.toml`) is:
```
xtensa-esp32-none-elf
```

Board type in `diagram.toml` is: 
```
"type": "wokwi-esp32-devkit-v1"
```
---
For `esp32s2` board target (and corresponding changes for paths in `wokwi.toml`) is:
```
xtensa-esp32s2-none-elf
```

Board type in `diagram.toml` is: 
```
"type": "board-esp32-s2-devkitm-1""
```
---
For `esp32s3` board target (and corresponding changes for paths in `wokwi.toml`) is:
```
xtensa-esp32s3-none-elf
```

Board type in `diagram.toml` is: 
```
"type": "board-esp32-s3-devkitc-1"
```

## Troubleshooting

* For `xtensa` targets you need to have `esp-idf` exported in your terminal
* In case of errors, related to `esp-hal` driver - try to play with dependencies versions on Cargo.toml. Sometimes there're a lot of breaking or light changes, that can affect building process

