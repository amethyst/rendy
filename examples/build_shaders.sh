#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

GLSLC=$($DIR/find_glslc.sh)

$GLSLC -MD -c -g -O -o examples/src/sprite/shader.vs examples/src/sprite/shader.vert
$GLSLC -MD -c -g -O -o examples/src/sprite/shader.fs examples/src/sprite/shader.frag

$GLSLC -MD -c -g -O -o examples/src/meshes/shader.vs examples/src/meshes/shader.vert
$GLSLC -MD -c -g -O -o examples/src/meshes/shader.fs examples/src/meshes/shader.frag