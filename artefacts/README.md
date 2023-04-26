# Adventure of a lifetime artefacts

The experiment can best be reproduced in a custom docker image we built with all the dependencies and PATH-setup.

Please ensure that you have docker setup and have enough permissions to use it.

To run the docker file, you can use this command:
```bash
sudo docker run -d -e USER=tester -e PASSWORD=tester -v /dev/shm:/dev/shm -p 6080:80 sewenthy/rem_test
```

Then visit http://localhost:6080 to access the VNC server (you might need to wait a while for it to launch).

## Extractions
