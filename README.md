# LazyLogger - WIP

TUI for viewing logs of AWS ECS services.

Current version allows the user to select an AWS profile, cluster, and
service and view the cloudwatch logs for that service.

## Linux Installation

Download release .tar.gz file from [releases](https://github.com/DillonKyle/lazylogger/releases)

```
curl -L https://github.com/DillonKyle/lazylogger/releases/download/1.0.1/lazylogger-1.0.1-x86_64-unknown-linux-musl.tar.gz 
> lazylogger.tar.gz
```

unzip and extract

```
tar -xvzf lazylogger.tar.gz
```

cd into the extracted folder and run

```
./lazylogger
```
