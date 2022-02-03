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
    docker build -f Dockerfile_pre_test -t dioxus-base-test-image .
    # run test
    docker build -f Dockerfile_test -t dioxus-test-image .

    # clean up
    rm -rf tmp
    if [ $# -ge 1 ]
    then
        echo "Got some parameter"
        if [ $1 = "--with-full-docker-cleanup" ]
        then
        docker image rm dioxus-base-test-image
        docker image rm dioxus-test-image
        docker system prune -af
        fi
    fi
}

run_script || echo "Error occured.. cleaning a bit." && \
    docker system prune -af;

docker system prune -af

echo "Script finished to execute"
