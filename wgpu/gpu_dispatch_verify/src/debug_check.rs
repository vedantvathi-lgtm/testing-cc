
# Just check what range_from returns with a quick inline test
cargo run 2>&1 | head -5

# Check: is range_from(1) returning 1..current_len including everything?
grep -n "range_from" /home/bb/project/Dave-Sonar-WGPU--master/wgpu/naga/src/arena/mod.rs | head -5
