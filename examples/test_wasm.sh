#/bin/bash

if [ "$1" != "release" ]; then
  if [ "$1" != "debug" ]; then
    echo "Must provide debug or release"; exit 1;
  fi
  KIND="debug"
else
  KIND="release"
fi

if [ -z $2 ]; then
  echo "Must provide the name of a example to run"; exit 1;
fi

./examples/build_wasm.sh $KIND || { echo 'Build failed' ; exit 1; }

cp examples/src/spirv_cross_wrapper_glsl.js target/generated-wasm/$KIND || { echo 'Unknown failure' ; exit 1; }
cp examples/src/spirv_cross_wrapper_glsl.wasm target/generated-wasm/$KIND || { echo 'Unknown failure' ; exit 1; }

pushd target/generated-wasm/$KIND  || { echo 'Unknown failure' ; exit 1; }
google-chrome-stable "http://127.0.0.1:8000/$2"
python3 ../../../examples/host.py || { echo "Python http server failure" ;  popd; exit 1; }
popd
exit 0