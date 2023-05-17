# Adventure of a lifetime artefacts

The experiment can best be reproduced in a custom docker image we built with all the dependencies and PATH-setup.

Please ensure that you have docker setup and have enough permissions to use it.

To run the docker file, you can use this command:
```bash
sudo docker run -d -e USER=tester -e PASSWORD=tester -v /dev/shm:/dev/shm -p 6080:80 sewenthy/rem_test:frozen
```

Then visit http://localhost:6080 to access the VNC server (you might need to wait a while for it to launch).

## Extractions
All the example cases are in `/home/tester/Desktop/artefact_sample_projects`.  We structure it such that each case is in its own folder within that directory.  

To test each case:  
1. open the IntelliJ IDE by double-clicking `/home/tester/Desktop/idea.sh`.  
2. Locate the file where we run using `git diff HEAD~1..` within the `lxterminal` for each of the folder to see exactly where we inserted the `/* START|END SELECTION */` comments.  There is exactly one place in one file we do this for each folder.  
3. Once you found this file, highlight the code between the comments and then goto `Refactor > Extract Method...` (or `Ctrl-Alt-M`), then enter any name you want and click `OK`.  
4. Depending on our comments, you see the expected success/failure and you can also manually verify the semantics of the extracted code.  

## WIP Features
Our de-sugaring of `?` are still WIP (e.g. we do not handle it well when it is within a closure within a function, etc).  

The type inferences are dependent on IntelliJ for now so it will not work sometimes.  

## Notes
Currently, there are some weird bugs with our usages of the syn crate which cause us to ignore comments in our extraction file.  So when you extract, some comments might be transformed into `#[doc]` attributes and some might be missing.  

Sometimes, running the terminal inside IntelliJ in the container hangs, so we suggest running it from the actual `lxterminal`.  

Running the program using IntelliJ run configuration seems to hang for no particular reason either.  We suggest to run `cargo run|check|build` using the `lxterminal`.
