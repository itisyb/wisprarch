# install

deploy using godeploy `godeploy deploy`

- this uses the godpeloy.config.json file. which points to the releaes dir to publish

the domain will be: <https://install.audetic.ai/>

the user will be able to run `curl -sSL https://install.audetic.ai/latest.sh | bash`

- a releases dir
- version file
- subfolder for each release version including the binary
- add subfolders to gitignore

- install script that detects os and installs

# auto-update

just like we spawn the api server in the main.rs, I want to spawn an auto update service that is written in rust. This service will check <https://install.audetic.ai/cli/version> periodically and if the version is different from the current version, it will download the new version and replace the current binary and restart the service.
We also want the audetic binary to be available in the users bin and path so we can use audetic cli from terminal to trigger other commands etc from the cli
