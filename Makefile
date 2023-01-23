.PHONY: p g t

p:
	cargo fmt
	# TODO add this cargo clippy -- -Dwarnings
	cargo clippy

t: p
	cargo test

# TODO add g: t (so it passes tests before you commit)
g: p
	git add .
	git commit -m "x"
	git push

