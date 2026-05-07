#!/bin/sh

cargo run --features=cli --bin=llw -- -o ../lelwel-codegen/src/frontend ../lelwel-codegen/src/frontend/lelwel.llw && rustfmt ../lelwel-codegen/src/frontend/generated.rs
