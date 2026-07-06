#!/bin/bash
# TopoFlow Quick Start Script
# Run this after building the project

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  TopoFlow Quick Start                                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo

BINARY="./target/release/topoflow"

if [ ! -f "$BINARY" ]; then
    echo "Building TopoFlow..."
    cargo build --release
    echo
fi

echo "Available commands:"
echo "  1. Analyze mesh:       $BINARY info <file.obj>"
echo "  2. Validate mesh:       $BINARY validate <file.obj>"
echo "  3. Auto-retopology:     $BINARY retopo <input.obj> <output.obj>"
echo "  4. Decimate:            $BINARY decimate <input.obj> <output.obj> --ratio 0.5"
echo "  5. Run tests:           cargo test"
echo

# Test with sample if available
if [ -f "assets/sample_sphere.obj" ]; then
    echo "Testing with sample sphere..."
    echo
    $BINARY info assets/sample_sphere.obj
fi
