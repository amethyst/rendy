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
                stage("Test on Linux") {
                    agent {
                        docker {
                            image 'amethystrs/builder-linux:stable'
                            label 'docker'
                        }
                    }
                    steps {
                        echo 'Beginning tests...'
                        sh 'cd rendy && cargo test --all --features "full vulkan"'
                        echo 'Tests done!'
                    }
                }
                stage('Coverage') {
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
                            sh 'cargo test --all'
                            echo 'Calculating code coverage...'
                            sh 'for file in target/debug/rendy*[^\\.d]; do mkdir -p \"target/cov/$(basename $file)\"; kcov --exclude-pattern=/.cargo,/usr/lib --verify \"target/cov/$(basename $file)\" \"$file\" || true; done'
                            echo "Uploading coverage..."
                            sh "curl -s https://codecov.io/bash | bash -s - -t $CODECOV_TOKEN"
                            echo "Uploaded code coverage!"
                        }
                    }
                }
            }
        }
    }
}
