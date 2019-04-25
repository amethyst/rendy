pipeline {
    agent none
    stages {
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
                    environment {
                        CARGO_HOME = '/home/jenkins/.cargo'
                        RUSTUP_HOME = '/home/jenkins/.rustup'
                    }
                    agent {
                        label 'linux'
                    }
                    steps {
                        echo 'Beginning tests...'
                        sh 'cargo test --all --features "full vulkan"'
                        echo 'Tests done!'
                    }
                }
            }
        }
    }
}
