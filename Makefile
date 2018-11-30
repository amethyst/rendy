RUST_BACKTRACE:=1
RENDY_FEATURES:=

ifeq ($(OS),Windows_NT)
	RENDY_FEATURES=dx12
else
	UNAME_S:=$(shell uname -s)
	ifeq ($(UNAME_S),Linux)
		RENDY_FEATURES=vulkan
	endif
	ifeq ($(UNAME_S),Darwin)
		RENDY_FEATURES=metal
	endif
endif

test:
	cd rendy && cargo test --all --features $(RENDY_FEATURES)

doc:
	cargo doc
