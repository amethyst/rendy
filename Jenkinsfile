pipeline {
    agent none
    stages {
        stage("Pull new images") {
            agent {
                label 'docker'
            }
            steps {
                sh 'docker pull amethystrs/builder-linux:stable'
                sh 'docker pull amethystrs/builder-linux:nightly'
            }
        }
        stage('Check Formatting') {
            environment {
                CARGO_HOME = '/home/jenkins/.cargo'
                RUSTUP_HOME = '/home/jenkins/.rustup'
                RUSTFLAGS = "-D warnings"
            }
            agent {
                label 'linux'
            }
            steps {
                echo 'Checking formatting...'
                sh '$CARGO_HOME/bin/cargo fmt -- --check'
            }
        }
        stage('Run Tests') {
            parallel {
                stage('Test on Linux / coverage') {
                    agent {
                        docker {
                            image 'amethystrs/builder-linux:stable'
                            args '--privileged'
                            label 'docker'
                        }
                    }
                    steps {
                        withCredentials([string(credentialsId: 'codecov_token', variable: 'CODECOV_TOKEN')]) {
                            echo 'Building to calculate coverage'
                            sh 'cd rendy && cargo test --all --all-features'
                            echo 'Calculating code coverage...'
                            sh 'for file in target/debug/rendy*[^\\.d]; do mkdir -p \"target/cov/$(basename $file)\"; kcov --exclude-pattern=/.cargo,/usr/lib --verify \"target/cov/$(basename $file)\" \"$file\" || true; done'
                            echo "Uploading coverage..."
                            sh "curl -s https://codecov.io/bash | bash -s - -t $CODECOV_TOKEN"
                            echo "Uploaded code coverage!"
                        }
                    }
                }
                stage("Test on Windows") {
                    environment {
                        CARGO_HOME = 'C:\\Users\\root\\.cargo'
                        RUSTUP_HOME = 'C:\\Users\\root\\.rustup'
                    }
                    agent {
                        label 'windows'
                    }
                    steps {
                        bat 'C:\\Users\\root\\.cargo\\bin\\cargo update'
                        echo 'Beginning tests...'
                        // TODO: Once we support DX12, we should switch to it from vulkan for windows
                        // FIXME: Can't test "full" because of problems with shaderc compilation on windows box
                        bat 'cd rendy && C:\\Users\\root\\.cargo\\bin\\cargo test --all --no-default-features --features "base mesh-obj texture-image texture-palette spirv-reflection serde-1 dx12 gl vulkan"'
                        echo 'Tests done!'
                    }
                }
                // stage("Test on macOS") {
                //     environment {
                //         CARGO_HOME = '/Users/jenkins/.cargo'
                //         RUSTUP_HOME = '/Users/jenkins/.rustup'
                //     }
                //     agent {
                //         label 'mac'
                //     }
                //     steps {
                //         echo 'Beginning tests...'
                //         sh 'cd rendy && /Users/jenkins/.cargo/bin/cargo test --all --all-features'
                //         echo 'Tests done!'
                //     }
                // }
            }
        }
    }
}
