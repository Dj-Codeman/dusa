# COLORS 
GREEN = \033[0;32m
RED = \033[0;31m
NC = \033[0m # No color

# The default target to run,
all: init user_creation build register clean done
dirty: build register done
update: build

# verify and or add dusa user 
# While building and testing theres a chance the user already already exists on the system

user_creation:
	@echo -e "${GREEN}**Creating dusa user**${NC}"
	@-/bin/groupadd --system dusa > /dev/null
	@-/bin/useradd dusa -s/bin/nologin -d/dev/null > /dev/null
	@echo -e "${GREEN}**USER AND GROUPS CREATED${NC}**"

init:
	@echo -e "${GREEN}**Creating Folders**${NC}"
	@-mkdir -pv /var/run/dusa
	@-mkdir -pv /var/dusa
	@-mkdir -pv /tmp/logger
	@chmod -v 777 /tmp/logger
	@chown -R dusa:dusa /var/run/dusa /var/dusa
	
build:
	@cargo update
	cargo test
	cargo build --release
	@mv -v ./target/release/server /usr/bin/dusad
	@mv -v ./target/release/client /usr/bin/dusa
	@chmod +x -v /usr/bin/dusad /usr/bin/dusa
	@echo -e "${GREEN} Setting additional permission on the applications${NC}"
	/bin/setcap cap_chown=ep /usr/bin/dusa
	/bin/setcap cap_chown=ep /usr/bin/dusad
	@echo -e "${GREEN}Application built!${NC}"

register:
	@echo -e "${GREEN}REGISTERING WITH SYSTEMD${NC}"
	@cp -v ./dusad.service /etc/systemd/system/dusad.service
	@systemctl daemon-reload
	systemctl enable dusad --now

done:
	@echo -e "${GREEN}dusa server and cli installed successfully!!!${NC}"
	@echo -e "To get started make sure the current user is a part of the dusa group, 'usermod -a -G <user> dusa'"

clean: 
	cargo clean