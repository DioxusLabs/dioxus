# Why this?

This part is used to test whole package before pushing it

# How to use it?

Just run in the folder:
`bash run_local_tests.sh`. If nothing fails, then you can push your code to the repo.
or run:
`bash run_local_tests.sh --with-full-docker-cleanup`
for cleaning up images as well

# How is it composed of?

  1. `Dockerfile_pre_test` will build the base image for the tests to be run into
  2. `Dockerfile_test` will run the actual tests based on 1.
  3. `run_local_tests.sh` to wrap this up

# Warning

The task requires some amount of CPU work and disk space (5GB per tests). Some clean up is included in the script.

# Requirements

 * [docker](https://docs.docker.com/engine/install/)
 * bash
 * rsync