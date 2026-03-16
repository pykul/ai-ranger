# Proto code generation for Python and Go.
#
# Rust code generation is handled by prost-build in agent/build.rs
# (idiomatic Rust: compile-time generation via cargo build, not make proto).
#
# Prerequisites:
#   pip install grpcio-tools
#   go install google.golang.org/protobuf/cmd/protoc-gen-go@latest

PROTO_SRC   := ranger/v1/events.proto ranger/v1/agent.proto
GEN_PYTHON  := gen/python
GEN_GO      := gen/go

.PHONY: all clean

all:               ## Regenerate protobuf bindings for Python and Go
	mkdir -p $(GEN_PYTHON) $(GEN_GO)
	python3 -m grpc_tools.protoc \
		-I. \
		--python_out=$(GEN_PYTHON) \
		--pyi_out=$(GEN_PYTHON) \
		$(PROTO_SRC)
	PATH="$$HOME/go/bin:$$PATH" protoc \
		-I. \
		--go_out=$(GEN_GO) \
		--go_opt=paths=source_relative \
		$(PROTO_SRC)

clean:             ## Remove all generated files
	rm -rf $(GEN_PYTHON) $(GEN_GO)
