.PHONY: p g t

p:
	cargo fmt
	cargo clippy

t: p
	cargo test

g: p
	git add .
	git commit -m "x"
	git push

