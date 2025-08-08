@echo off
cd frontend
call trunk build
cd ..
cargo run