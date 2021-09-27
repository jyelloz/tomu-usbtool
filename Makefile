PROG = tomu-usbtool
BIN = ../../target/thumbv6m-none-eabi/release/$(PROG)

all: $(PROG).dfu

$(BIN):
	cargo build --release

$(PROG).bin:
	cargo objcopy --release --bin $(PROG) -- -O binary $@

$(PROG).ihex: $(BIN)
	arm-none-eabi-objcopy -O ihex $^ $@

$(PROG).dfu: $(PROG).bin
	cp $^ $@
	dfu-suffix -v 1209 -p 70b1 -a $@

clean:
	rm -fv $(PROG).{bin,dfu,ihex}

install: $(PROG).dfu
	dfu-util -D $^

.PHONY: install clean default all $(PROG).bin $(BIN)
