set -eux

echo "Test script started"

function run_script {
    if [[ -d tmp ]]
    then
        rm -rf tmp
    fi
    mkdir tmp
    # copy files first
    rsync -a --progress ../ tmp --exclude target --exclude docker

    # build base image
    docker build -f Dockerfile_base_test_image -t dioxus-base-test-image .
    docker build -f Dockerfile_pre_test -t dioxus-pre-test .
    # run test
    docker build -f Dockerfile_test -t dioxus-test-image .
    # code coverage
    docker build -f Dockerfile_code_coverage -t dioxus-code-coverage .

    # exec test coverage
    cd .. && \
    echo "rustup default nightly && cargo +nightly tarpaulin --verbose --all-features --tests --workspace --exclude core-macro --timeout 120 --out Html" | docker run -i --rm --security-opt seccomp=unconfined -v "/home/elios/project/prs/dioxus/:/run_test" dioxus-code-coverage

    # clean up
    rm -rf tmp
    if [ $# -ge 1 ]
    then
        echo "Got some parameter"
        if [ $1 = "--with-full-docker-cleanup" ]
        then
        docker image rm dioxus-base-test-image
        docker image rm dioxus-test-image
        fi
    fi
}

run_script || echo "Error occured.. cleaning a bit." && \
    docker system prune -f;

docker system prune -f

echo "Script finished to execute"
