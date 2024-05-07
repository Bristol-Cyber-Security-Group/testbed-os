# this script required the poetry environment to exist from running "poetry install" in this directory

UI_TUI_PATH="/home/$USER/.local/share/kvm-ui-tui"
# delete pre-existing python code if exists
[ -d "$UI_TUI_PATH" ]; rm -rf UI_TUI_PATH
# install kvm-ui-tui
mkdir -p $UI_TUI_PATH && cp TUI.py $UI_TUI_PATH
# create symlink to kvm-ui-tui script to a location in PATH
sudo ln -sf "$UI_TUI_PATH/TUI.py" "/usr/local/bin/kvm-ui-tui"

# update the shebang in the kvm-ui-tui script to have poetry venv

# get the location of python from the poetry venv
POETRY_VENV=$(poetry run which python)
# replace forward slash with escaped forward slash for sed
ESCAPED_POETRY_VENV=${POETRY_VENV//\//\\/}
sed -i "s/#!\/usr\/bin\/env python/#!$ESCAPED_POETRY_VENV/g" "$UI_TUI_PATH/TUI.py"
