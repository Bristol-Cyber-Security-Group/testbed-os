$(document).ready(function() {

    let selectedOption = '';
    let selectedSnapshotOption = '';
    let selectedToolOption = '';

    $('#execSubcommandMenu').on('click', 'a.dropdown-item', function() {
        selectedOption = $(this).attr('data-value');
    });

    $('#subcommandMenu').on('click', 'a.dropdown-item', function() {
        selectedSnapshotOption = $(this).attr('data-value');
    });

    $('#execDynamicButtons').on('change', '#toolOptionsDropdown', function() {
        selectedToolOption = $(this).val();
    });

    let cancel_button = $('#cancelCommandButton');
    cancel_button.prop("disabled", true);

    $('#subcommandMenu').on('click', 'a', function() {
        $('#dynamicButtons').empty();   // Clear existing checkboxes
    
        // Define options for commands
        const options = {
            'create': [
                {label: 'Guest name', type: 'guest', id: 'name'},
                {label: 'Snapshot ID', type: 'text', id: 'snapshot'},
                {label: 'Apply to all guests', type: 'checkbox', id: 'all'}
            ],
            'delete': [
                {label: 'Guest name', type: 'guest', id: 'name'},
                {label: 'Snapshot ID', type: 'text', id: 'snapshot'},
                {label: 'Apply to all snapshots', type: 'checkbox', id: 'all'}
            ],
            'info': [
                {label: 'Guest name', type: 'guest', id: 'name'}
            ],
            'list': [
                {label: 'Guest name', type: 'guest', id: 'name'},
                {label: 'Apply to all guests', type: 'checkbox', id: 'all'}
            ],
            'restore': [
                {label: 'Guest name', type: 'guest', id: 'name'},
                {label: 'Snapshot ID', type: 'text', id: 'snapshot'},
                {label: 'Apply to all guests', type: 'checkbox', id: 'all'}
            ]
        };
    
        // Get the selected command
        const selectedCommand = $(this).data('value');
        const selectedCommandText = $(this).text();
        $('#subcommandDropdown').text(selectedCommandText).append(' <span class="caret"></span>');
        generateInputs(options[selectedCommand] || [], '#dynamicButtons', true);
    });

    const toolOptions = {
        'a_d_b': [
            {label: 'Command', type: 'text', id: 'command'}
        ],
        'frida_setup': [],
        'test_permissions': [
            {label: 'Command', type: 'text', id: 'command'}
        ],
        'test_privacy': [
            {label: 'Command', type: 'text', id: 'command'}
        ],
        't_l_s_intercept': [
            {label: 'Command', type: 'text', id: 'command'}
        ]
    };
    
    $('#execSubcommandMenu').on('click', 'a', function() {
        let execDynamicButtons = $('#execDynamicButtons');
        execDynamicButtons.empty();   // Clear existing inputs
    
        const selectedCommand = $(this).data('value');
        const selectedCommandText = $(this).text();
        $('#execSubcommandDropdown').text(selectedCommandText).append(' <span class="caret"></span>');
    
        if (selectedCommand === 'tool') {
            // Create and append the second dropdown for tool options
            const toolDropdown = $('<select></select>', {
                class: 'form-select',
                id: 'toolOptionsDropdown'
            }).append($('<option>', {
                value: '',
                text: 'Select Tool Option'
            }));
    
            // Populate the second dropdown with options
            $.each(toolOptions, (key, _) => {
                toolDropdown.append($('<option>', {
                    value: key,
                    text: key
                }));
            });
    
            $('#execDynamicButtons').append(toolDropdown);
    
            // Handle changes to the second dropdown
            $('#toolOptionsDropdown').change(function() {
                const selectedToolOption = $(this).val();
                const optionsToDisplay = toolOptions[selectedToolOption] || [];
                $('#execDynamicButtons').find('div').remove();
    
                generateInputs(optionsToDisplay, '#execDynamicButtons', false);

                // add device dropdown
                const deviceTextRow = addDeviceDropDown();
                // TODO this is added as part of the form, while it works, the server is sent the device name twice,
                //  once properly and once again inside the tool under "deviceName" which is currently ignored
                //  this should be fixed
                execDynamicButtons.append(deviceTextRow);
            });
        } else {
            const options = {
                'shell_command': [
                    {label: 'Command', type: 'text', id: 'command'}
                ],
                'user_script': [
                    {label: 'Script', type: 'text', id: 'script'},
                    {label: 'Run on main', type: 'checkbox', id:'run_on_master'}
                ]
            };
            generateInputs(options[selectedCommand] || [], '#execDynamicButtons', false);

            // add device drop down
            const deviceTextRow = addDeviceDropDown();
            execDynamicButtons.append(deviceTextRow);
        }
    });
    
    function generateInputs(options, dynamicButtonsId, applyDisableLogic) {
        let checkboxFound = false;
    
        options.forEach((option, index) => {
            const inputFieldId = option.id;
    
            if (option.type == 'checkbox') {
                checkboxFound = true;
                const checkboxDiv = createCheckbox(option, inputFieldId);
                $(dynamicButtonsId).append(checkboxDiv);
            } else if (option.type == 'text') {
                const textRow = createTextInput(option, inputFieldId);
                $(dynamicButtonsId).append(textRow);
            }else if (option.type == 'guest') {
                const checkboxDiv = addDeviceDropDown();
                $(dynamicButtonsId).append(checkboxDiv);
            }
        });
    
        // Attach a change event listener to checkboxes if applyDisableLogic is true
        if (checkboxFound && applyDisableLogic) {
            $(dynamicButtonsId).on('change', 'input[type="checkbox"]', function() {
                const isCheckboxChecked = $(this).is(':checked');
                $(dynamicButtonsId + ' input[type="text"]').each(function() {
                    $(this).prop('disabled', isCheckboxChecked);
                });
            });
        }
    }

    function addDeviceDropDown() {
        // all tooling needs to have be assigned to a tool, this will add a textbox to enter the name
        // TODO this should eventually be filtered by guest type compatible with tool
        const textRow = $('<div></div>', {class: 'row'});
        const textLabelCol = $('<div></div>', {class: 'col-auto'});
        const textInputCol = $('<div></div>', {class: 'col'});

        const textLabel = `Guest Name  `;
        const textLabelElement = $('<label></label>', {
            for: 'guestName',
            text: textLabel,
            class: 'form-label'
        });

        const selectInput = $('<select>', {
            class: 'form-control form-control-sm',
            id: 'guestName',
            style: 'cursor: pointer;'
        });

        // Fetch state JSON to dynamically add available guests to dropdown menu
        $.get(server_url + '/api/deployments/' + project_name + '/state?pretty=true', function(statejson) {
            const guests = statejson.testbed_guests;
            const guestNames = Object.keys(guests).map(key => guests[key].name);
            console.log(guestNames);

            guestNames.forEach(function(guest) {
                selectInput.append($('<option>', { text: guest }));
            });

            textInputCol.append(selectInput);
        });

        textLabelCol.append(textLabelElement);
        return textRow.append(textLabelCol).append(textInputCol);
    }
    
    function createCheckbox(option, inputFieldId) {
        const checkboxDiv = $('<div></div>', {class: 'col'});
        const checkboxInput = $('<input>', {
            type: 'checkbox',
            class: 'form-check-input',
            id: inputFieldId,
            value: ''
        });
        const checkboxLabel = `Flag: ${option.label}  `;
        const checkboxLabelElement = $('<label></label>', {
            for: inputFieldId,
            text: checkboxLabel,
            class: 'form-check-label'
        });
    
        return checkboxDiv.append(checkboxLabelElement).append(checkboxInput);
    }
    
    function createTextInput(option, inputFieldId) {
        const textRow = $('<div></div>', {class: 'row'});
        const textLabelCol = $('<div></div>', {class: 'col-auto'});
        const textInputCol = $('<div></div>', {class: 'col'});
    
        const textLabel = `Argument: ${option.label}  `;
        const textLabelElement = $('<label></label>', {
            for: inputFieldId,
            text: textLabel,
            class: 'form-label'
        });
        const textInput = $('<input>', {
            type: 'text',
            class: 'form-control form-control-sm',
            id: inputFieldId,
            value: ''
        });
    
        textLabelCol.append(textLabelElement);
        textInputCol.append(textInput);
        return textRow.append(textLabelCol).append(textInputCol);
    }

    $('.orchestration-button').click(function (event) {
        // TODO - if something is already running, prevent triggering another command

        // get the button press of a button to create the command json
        let command_json = get_button_command_json(event['currentTarget']['id'], selectedOption, selectedToolOption, selectedSnapshotOption);
        // console.log(JSON.stringify(command_json));
        request_command(command_json);
    });

    // TODO - cancel command button

});

// function to send a prebuilt command to the server, this needs a new endpoint on the server
// that will do the orchestration itself to itself (reusing cli code)

function request_command(init_schema) {

    // keep track of connection state
    let connection_state = $('#pseudoTerminalConnectionState');

    // if a command is already running, stop
    // TODO - a better way, this is just looking at a side effect of running a command, not great
    if (connection_state.hasClass('text-bg-primary')) {
        $('#commandAlreadyRunningModal').modal('show');
    }

    // open websocket
    let websocket_url = server_url.replace("http://", "ws://");
    let command_generator_websocket = new WebSocket(websocket_url + '/api/orchestration/gui');

    // store the messages from the server, before sending them back. this is so we don't have to concurrently manage
    // two websockets to the server
    let generated_commands = [];

    // keep track of pseudo terminal div
    let terminal = $('#pseudoTerminal');

    // keep track of the cancel button
    let cancel_button = $('#cancelCommandButton');
    // enable cancel button
    cancel_button.prop("disabled", false);

    // track if the command generation was successful
    let command_generation_failure = false;

    // we must wait for the socket to be open before sending and receiving, specifically on open, send the
    // init command but defer to the onmessage to handle the processing on the web page
    command_generator_websocket.onopen = function (event) {
        // clear the previous log output
        terminal.html('');

        // console.log('websocket is open');

        // send the init command in json format to match the 'GUICommand' JSON schema
        command_generator_websocket.send(JSON.stringify(init_schema));

        // set state badge to active
        set_connection_state_active(connection_state);

        // set state to running as the state on server won't have updated yet
        $('#displayState').html('RUNNING');
    }

    command_generator_websocket.onmessage = function (event) {
        // process the server messages
        let server_response = event.data;
        let server_message = JSON.parse(server_response);
        // console.log('received response from server:', confirmation_message);

        // console.log(server_message);

        // TODO - handle init confirmation, normal message, interactive question to user, close message

        // establish a connection to the server command generator
        if (server_message.hasOwnProperty("init_msg")) {
            // always try to handle init first
            if (server_message['init_msg'] === true) {
                // make sure the server understood the request
                if (server_message['was_error'] === false) {
                    append_terminal_text(terminal, "Executing the following command: <br>" + server_message['message']);
                } else {
                    append_terminal_text(terminal, "Command sent to server was not understood: <br>" + server_message['message']);
                    command_generation_failure = true;
                }

            } else {
                // general messages from server
                // TODO - can we format the ISO timestamp and up to but including INFO/ERROR in a different colour?
                if (server_message['was_error'] === false) {
                    append_terminal_text(terminal, server_message['message']);
                } else {
                    append_terminal_text(terminal, server_message['message'] + '<br>Please see server logs for more details.');
                    command_generation_failure = true;
                }
            }
        } else {
            // is not a command generator init message, now send protocol to server for orchestration

            // console.log("should be sending protocol");
            // send message
            // append_terminal_text(terminal, "sending command: " + JSON.stringify(server_message));
            // var arrayBuffer = new TextEncoder().encode(server_message).buffer;
            // var blob = new Blob([arrayBuffer], { type: 'application/octet-stream' });
            // command_generator_websocket.send(blob);
            // get acknowledgement from server

            // get command run confirmation

            // store each command from server
            // console.log("storing generated command");
            generated_commands.push(server_message);

        }

    }

    command_generator_websocket.onclose = function (event) {
        console.log("closed command generator socket");
    }

    // once the command generation socket closed, lets run the next socket
    command_generator_websocket.addEventListener('close', function () {
        // console.log("generated commands: " +generated_commands);
        if (command_generation_failure === false) {
            run_orchestration(generated_commands, websocket_url, terminal, connection_state);
        } else {
            append_terminal_text(terminal, "Due to an error in command generation, will not continue to orchestration.");
            reset_running_state(connection_state);
        }
    })

}

function run_orchestration(generated_commands, websocket_url, terminal, connection_state) {
    console.log("running orchestration");
    // generated_commands.forEach((element) => console.log(element));
    console.log(websocket_url + '/api/orchestration/ws');
    let command_runner_websocket = new WebSocket(websocket_url + '/api/orchestration/ws');

    // TODO - the websockets are not being cleans up properly on the clientside and serverside

    // TODO - for each send, we need to wait for two replies before sending the next. So that we can also then maybe
    //  interrogate the user for an option or cancel request

    // keep track of replies, we only send after two replies
    let messages_received = 0;
    // track which message we are sending
    let current_command = 0;
    let n_commands = generated_commands.length;

    function send_message(generated_commands, msg_n) {
        // console.log("sending: " + JSON.stringify(generated_commands[msg_n]));

        // TODO - print to terminal formatted command here
        formatServerSendMessages(terminal, generated_commands[msg_n])

        var arrayBuffer = new TextEncoder().encode(JSON.stringify(generated_commands[msg_n])).buffer;
        var blob = new Blob([arrayBuffer], { type: 'application/octet-stream' });
        command_runner_websocket.send(blob);
    }

    function handle_message(event) {

        // for the protocol between the client and server, for each command sent to the server it will reply twice ...
        // the first reply is an acknowledgement of the command, and the second reply is the outcome of the command,
        // so that could be a success/fail
        // therefore, this handle message function will count the number of messages received, such that when it reaches
        // two messages, it will reset back to 0 and send the next command

        // there is now one more message type, which is just a log message during the command that might be useful to
        // the user, and this should not increment the message count

        var message = event.data;
        // console.log('Received message:', message);
        if (message instanceof Blob) {
            // there should not be any more blob messages
            append_terminal_text(terminal, "Error: received a blob message from server, this is not supported, please raise a GitHub issue");
            command_runner_websocket.close();

        } else if (message.includes("{\"Generic\":{\"is_success\":false,\"message\":\"Cancel request\"}}")) {
            // handle the cancel response differently to other messages

            append_terminal_text(terminal, "Command has been cancelled, end of command running.");
            command_runner_websocket.close();
            return;

        } else {

            // logs anything that is not a binary blob from server
            if (message === "Receiving instruction OK") {
                // don't print this message for now
            } else {
                // need to take current command -1 as we still need to process the init response in the first time this
                // function is called
                formatServerResponseMessages(terminal, message, generated_commands[current_command-1]);
            }
        }

        // if the message is just a logging message from the server, don't increment message count
        // this is a bit rubbish way of checking if it is a log message, should refactor
        if (message.includes("{\"Log\":{")) {
            let json_message = JSON.parse(message);
            let log_level = json_message["Log"]["level"];
            let log_message = json_message["Log"]["message"];
            // just log the message in the terminal
            if (log_level === "Info") {
                append_terminal_text(terminal,log_text_with_colour(true, log_message));
            } else if (log_level === "Error") {
                append_terminal_text(terminal,log_text_with_colour(false, log_message));
            }
        } else {
            // increment the message receive count and check if we need to send the next command to the server
            messages_received++;

            if (messages_received === 2) {
                if (current_command === n_commands) {
                    console.log("we have reached the end of commands, closing");
                    append_terminal_text(terminal, "End of command running.");
                    command_runner_websocket.close();
                } else {
                    // TODO we can now send the next message to the server
                    send_message(generated_commands, current_command);
                    current_command++;
                    // reset counter
                    messages_received = 0;
                }
            }
        }
    }

    // handle the messages from server
    command_runner_websocket.addEventListener('message', handle_message);

    // send the first message once socket opens
    command_runner_websocket.addEventListener('open', function () {
        send_message(generated_commands, current_command);
        current_command++;
    });

    // add cancel button handler, send Cancel protocol json
    $('#cancelCommandButton').on('click', function() {
        // console.log("sending cancel request");
        var arrayBuffer = new TextEncoder().encode(JSON.stringify(
            {
                "instruction": "Cancel"
            }
        )).buffer;
        var blob = new Blob([arrayBuffer], { type: 'application/octet-stream' });
        command_runner_websocket.send(blob);
    });


    command_runner_websocket.onclose = function (event) {
        console.log("closing orchestration websocket");

            // handle when the server closed the websocket, this includes expected and unexpected closes
        append_terminal_text(terminal, '<br>' + event['reason']);

        // set state badge to inactive
        reset_running_state(connection_state);
    }


    // command_runner_websocket.close();

}

function reset_running_state(connection_state) {
    // set state badge to inactive
    set_connection_state_inactive(connection_state);

    // disable cancel button
    let cancel_button = $('#cancelCommandButton');
    cancel_button.prop("disabled", true);

    // check deployment page and check for state
    get_last_command_state();
}

function blobToString(blob) {
    return new Promise((resolve, reject) => {
        // Create a new FileReader object
        var reader = new FileReader();

        // Define the onload event handler for the FileReader
        reader.onload = function(event) {
            // event.target.result contains the string representation of the blob
            var blobString = event.target.result;
            resolve(blobString);
        };

        // Define the onerror event handler for the FileReader
        reader.onerror = function(event) {
            reject(event);
        };

        // Read the contents of the Blob as text
        reader.readAsText(blob);
    });
}


/// Helper to push text into the pseudo terminal, and automatically scroll to the bottom to follow the messages as
/// they fill the div
function append_terminal_text(terminal_div, msg) {
    // if this is a log line with the timestamp and log level, change it's colour
    // if (msg.includes("INFO:")) {
    //     let start = msg.indexOf("INFO:");
    //     let end = start + 5;
    //     msg = msg.slice(0, start) + '<span style="color: #346beb">' + msg.slice(start, end) + '</span>' + msg.slice(end);
    // }

    // TODO - colour logs
    // let coloured_msg = info_colour_log_text(msg);
    // coloured_msg = error_colour_log_text(coloured_msg);

    // TODO - this could be very inefficient with large logs, can we append to the text instead?
    let terminal_text = terminal_div.html();
    terminal_text = terminal_text + msg + "<br>";
    terminal_div.html(terminal_text);

    terminal_div.each( function() {
        let scrollHeight = Math.max(this.scrollHeight, this.clientHeight);
        this.scrollTop = scrollHeight - this.clientHeight;
    });
}

function set_connection_state_active(connection_state) {
    connection_state.removeClass("text-bg-secondary");
    connection_state.addClass("text-bg-primary");
    connection_state.html('Command Runner Active');
}

function set_connection_state_inactive(connection_state) {
    connection_state.removeClass("text-bg-primary");
    connection_state.addClass("text-bg-secondary");
    connection_state.html('Command Runner Inactive');
}

function log_text_with_colour(is_success, msg) {

    // were going to fudge the timestamp, it will be slightly different to the server, but even if we generated this in
    // the server, it also would not be the same as the server logs, although closer than when generated here
    const date = new Date();
    const time_stamp = date.toISOString();

    let log_level;
    if (is_success) {
        log_level = '<span style="color: #21730d">  INFO: </span>';
    } else {
        log_level = '<span style="color: #b31e0b"> ERROR: </span>';
    }

    return time_stamp + log_level + msg;

}

function get_last_command_state() {
    $.ajax({
        url: server_url + '/api/deployments/' + project_name,
        type: 'GET',
        success: function (result_state) {
            // console.log(result_state);
            let json = result_state;
            if (json['state'] === 'up') {
                $('#displayState').html('UP');
            } else if (json['state'] === 'down') {
                $('#displayState').html('DOWN');
            } else if (json['state'] === 'running') {
                $('#displayState').html('RUNNING');
            } else if (json['state']['failed'] !== undefined) {
                $('#displayState').html('CMD FAILED');
            } else {
                $('#displayState').html(json['state']);
            }
        },
        error: function (error) {
            console.log('was not able to get last command state for ' + project_name + ' error: ' + error);
        }
    });
}

function get_button_command_json(button_id, selectedOption, selectedToolOption, selectedSnapshotOption) {
    // depending on the button, get the relevant data from options/dropdown to build the command JSON
    // to be sent to the server

    let command_json = {
        "project_name": project_name,
        "sub_command": null,
    };
    let sub_command = null;

    // check each button id
    if (button_id === 'upButton') {
        // console.log('up');

        sub_command = {
            "Up": {
                "provision": $('#upProvisionFlag').is(':checked'),
                "rerun_scripts": $('#upReRunScriptsFlag').is(':checked'),
                "reapply_acl": $('upReRunACLFlag').is(':checked'),
            }
        };

    } else if (button_id === 'downButton') {
        // console.log('down');

        sub_command = "Down";

    } else if (button_id === 'snapshotButton') {
        let dynamicInputs = {}; 
        
        // Check if there are text inputs or checkboxes within the execDynamicButtons container
        $('#dynamicButtons input').each(function() {
            let inputType = $(this).attr('type');
            let inputValue = $(this).val();
            let inputId = $(this).attr('id');
            
            if(inputType === 'checkbox') {
                dynamicInputs[inputId] = $(this).is(':checked');
            } else if(inputType === 'text') {
                dynamicInputs[inputId] = inputValue;
            }
            
        });

        sub_command = {
            "Snapshot": {
                "sub_command": {
                    [selectedSnapshotOption]: {
                        ...dynamicInputs,
                    }
                }
            }
        };

    } else if (button_id === 'generateArtefactsButton') {
        // console.log('generate artefacts');

        sub_command = "GenerateArtefacts";

    } else if (button_id === 'clearArtefactsButton') {
        // console.log('clear artefacts');

        sub_command = "ClearArtefacts";

    } else if (button_id === 'testbedSnapshotButton') {
        // console.log('testbed snapshot');

        sub_command = {
            "TestbedSnapshot": {
                "snapshot_guests": $('#snapshotGuestsFlag').is(':checked'),
            }
        }

    } else if (button_id === 'listCloudImagesButton') {
        // console.log('cloud images');

        sub_command = "CloudImages";

    } else if (button_id === 'analysisToolsButton') {
        // TODO - not yet implemented, this will be commands like TCP dump
        console.log('analysis tools');
    } else if (button_id === 'execButton') {
        // console.log('exec');
        // everything in this sub_command is snake case, rather than pascal case like the other commands or outer part

        let dynamicInputs = {}; 
        
        // Check if there are text inputs or checkboxes within the execDynamicButtons container
        $('#execDynamicButtons input').each(function() {
            let inputType = $(this).attr('type');
            let inputValue = $(this).val().trim().replace(/\s+/g, ' '); // Remove whitespace from start and end of command string
            let inputId = $(this).attr('id');
            
            if(inputType === 'checkbox') {
                dynamicInputs[inputId] = $(this).is(':checked');
            } else if(inputType === 'text') {
                dynamicInputs[inputId] = [inputValue];
            }
            
        });
        
        // Construct sub_command based on selected tool and collected inputs.
        if(selectedOption == "tool"){
            if(Object.keys(dynamicInputs).length > 0){
                sub_command = {
                    "Exec": {
                        "guest_name": toolingGetGuestName(),
                        "command_type": {
                            "tool": {
                                "tool":{
                                    [selectedToolOption]: {
                                        ...dynamicInputs,
                                    }
                                }
                            }
                        }
                    }
                };
                // specific edits for some commands
                // TODO - the way this has been written assumes the same nested format for all, but each command
                //  can be slightly different. for now just do an edit on top but in the future we should re-write this
                //  so that each command is handled with its own parameters

                // the commands need special treatment, the text string for the command needs to be split into a
                // list of strings
                if ("a_d_b" in sub_command["Exec"]["command_type"]["tool"]["tool"]) {
                    convertCommandToListOfStrings(sub_command, "a_d_b");
                }
                if ("test_permissions" in sub_command["Exec"]["command_type"]["tool"]["tool"]) {
                    convertCommandToListOfStrings(sub_command, "test_permissions");
                }
                if ("test_privacy" in sub_command["Exec"]["command_type"]["tool"]["tool"]) {
                    convertCommandToListOfStrings(sub_command, "test_privacy");
                }
                if ("t_l_s_intercept" in sub_command["Exec"]["command_type"]["tool"]["tool"]) {
                    convertCommandToListOfStrings(sub_command, "t_l_s_intercept");
                }

                // frida setup has no parameters, just specify the tool directly
                if ("frida_setup" in sub_command["Exec"]["command_type"]["tool"]["tool"]) {
                    sub_command["Exec"]["command_type"]["tool"]["tool"] = "frida_setup";
                }

            }else{
                sub_command = {
                    "Exec": {
                        "guest_name": toolingGetGuestName(),
                        "command_type": {
                            "tool": {
                                "tool": selectedToolOption,
                            }
                        }
                    }
                };
            }
        }else{
            sub_command = {
                "Exec": {
                    "guest_name": toolingGetGuestName(),
                    "command_type": {
                        [selectedOption]: {
                            ...dynamicInputs,
                        }
                    }
                }
            };
        }

    } else {
        console.log(button_id + ' not recognised');
    }

    command_json['sub_command'] = sub_command;
    return command_json;
}

function toolingGetGuestName() {
    // for the tooling commands, get the device name
    return $('#guestName').val();
}

function convertCommandToListOfStrings(sub_command, command_name) {
    const regex = /(?:[^\s"]+|"[^"]*")+/g; // Regex to match quoted strings or individual words
    let command = sub_command["Exec"]["command_type"]["tool"]["tool"][command_name]["command"];
    
    // command starts as a list of one string
    let command_string = command[0];
    let command_list = command_string.match(regex).map(arg => arg.replace(/(^"|"$)/g, ''));
    
    // add formatted command back to json
    sub_command["Exec"]["command_type"]["tool"]["tool"][command_name]["command"] = command_list;
    
    return sub_command;
}

function formatServerSendMessages(terminal, command_json) {
    // print in the pseudo terminal a prettyfied version of what the GUI just sent to the server as a command

    // console.log(command_json);
    // console.log(JSON.stringify(command_json));

    // the format of the json sent will be one outer key with the instruction name i.e. Destroy or Deploy or TestbedHostCheck etc
    // these may or may not have further elements describing what is being done i.e. Destroy with a list of guests

    // we just want to print the command and the target, for example:
    // making request: Testbed Host Check
    // making request: Destroy [ Docker guest nginx, Android guest phone ]
    // making request: Destroy Temporary Network
    // making request: Destroy Ovn Logical Switch avd-sw0

    // we have to re-create some of the formatting CLI would apply to this, so we will do a best fit as this is only
    // for a visual guide to the GUI user to see what is going on during the command

    // first, get the instruction name, it could be just a string instead of a complex json
    let instruction = getInstructionName(command_json);

    // console.log("instruction: " + instruction);
    // next, we get some information about the command to put after the instruction
    let context;
    if (typeof command_json["instruction"] !== "string") {
        context = extractContextFromCommandJson(command_json);
    } else {
        context = "";
    }

    // log to terminal
    let formatted_text = "making request: " + instruction + " " + context;
    formatted_text = log_text_with_colour(true, formatted_text);
    append_terminal_text(terminal, formatted_text);

}

function formatServerResponseMessages(terminal, text, last_command) {
    // print in the pseudo terminal a prettified response from the command to the server

    let instruction = getInstructionName(last_command);

    try {
        let json_message = JSON.parse(text);
        // is a json, now we need to handle the different types ...
        // there is the enum in api.rs `OrchestrationProtocolResponse` which has three states:
        // Generic, Single and List ... this is signalled as the first key in the json

        let formatted_text;
        switch (true) {
            case "Generic" in json_message:
                // example: {"Generic":{"is_success":true,"message":"Running Init"}}

                // TODO - special case for cancel request?

                formatted_text = log_text_with_colour(json_message["Generic"]["is_success"], json_message["Generic"]["message"]);
                append_terminal_text(terminal, formatted_text);
                break;
            case "Single" in json_message:
                // example {"Single":{"is_success":true,"message":"Testbeds are up"}}
                formatted_text = log_text_with_colour(json_message["Single"]["is_success"], json_message["Single"]["message"]);
                append_terminal_text(terminal, formatted_text);
                break;
            case "List" in json_message:
                // example: {"List":[{"is_success":true,"message":"Ovn Logical Router Port avd-lr0-sw0"}]}
                // separate out list and format into one message

                // check if any in list failed
                let successful = [];
                let failed = [];

                json_message["List"].forEach(function (xx) {
                    if (xx["is_success"]) {
                        successful.push(xx["message"]);
                    } else {
                        failed.push(xx["message"]);
                    }
                })

                // print each, if any, if only one in list don't add brackets
                if (successful.length > 0) {
                    let list_text;
                    if (successful.length === 1) {
                        list_text = successful.join(", ");
                    } else {
                        list_text = "[ " + successful.join(", ") + " ]";
                    }
                    // TODO - rather than OK, get the instruction i.e. Destroy or Deploy etc
                    list_text = instruction + " Ok: " + list_text
                    formatted_text = log_text_with_colour(true, list_text);
                    append_terminal_text(terminal, formatted_text);
                }
                if (failed.length > 0) {
                    let list_text;
                    if (successful.length === 1) {
                        list_text = failed.join(", ");
                    } else {
                        list_text = "[ " + failed.join(", ") + " ]";
                    }
                    list_text = "Fail: " + list_text
                    formatted_text = log_text_with_colour(false, list_text);
                    append_terminal_text(terminal, formatted_text);
                }
                break;
            case "Log" in json_message:
                console.error("Should not be parsing the 'Log' message here");
                break;
            default:
                console.error("The json message from the server did not match the three variants in OrchestrationProtocolResponse");
                break;
        }

    } catch (e) {
        console.log("@ message was not a JSON = "+text);
        append_terminal_text(terminal, text);
    }

}

function getInstructionName(command_json) {
    let instruction;
    if (typeof command_json["instruction"] === "string") {
        instruction = command_json["instruction"];
    } else {
        instruction = Object.keys(command_json["instruction"]);
    }
    return instruction;
}

function extractContextFromCommandJson(instruction_json) {
    // since we don't have the schema and the helpers in the rust code for `OrchestrationInstruction` in this JS code,
    // we need to do a bit of code duplication - this will be a minimal bit of code to get the resource and name to
    // be printed in the pseudoterminal
    // some of the instructions don't have any sub elements such as `Init` and `Setup`, so we don't need to write any
    // context for those
    // some we won't give context for such as `CreateTempNetwork`, doesn't add much to the user experience

    // TODO - these are often a list after the instruction

    let message;
    let message_list = [];
    switch (true) {
        // case "Init" in instruction_json["instruction"]:
        //     break;
        // case "SetupImage" in instruction_json["instruction"]:
        //     break;
        // case "PushArtefacts" in instruction_json["instruction"]:
        //     break;
        // case "PushBackingImages" in instruction_json["instruction"]:
        //     break;
        // case "RebaseRemoteBackingImages" in instruction_json["instruction"]:
        //     break;
        case "Deploy" in instruction_json["instruction"]:
            message = extractNameForAllResources(instruction_json["instruction"]["Deploy"]);
            break;
        case "Destroy" in instruction_json["instruction"]:
            message = extractNameForAllResources(instruction_json["instruction"]["Destroy"]);
            break;
        // case "Edit" in instruction_json["instruction"]:
        //     break;
        // case "RunSetupScripts" in instruction_json["instruction"]:
        //     break;
        // case "Snapshot" in instruction_json["instruction"]:
        //     break;
        // case "AnalysisTool" in instruction_json["instruction"]:
        //     break;
        // case "Exec" in instruction_json["instruction"]:
        //     break;
        default:
            // leave this an empty string, add above to catch more if more information should be presented to the user
            message = "";
            break;
    }

    return message;
}

function extractNameForAllResources(resource_json) {
    let message;
    if (Array.isArray(resource_json)) {
        let message_list = [];
        resource_json.forEach((instruction) => {
            message_list.push(extractNameForOneResource(instruction));
        });
        message = "[ " + message_list.join(", ") + " ]";
    } else {
        message = extractNameForOneResource(resource_json);
    }
    return message;
}

function extractNameForOneResource(resource_json) {
    // each resource may be a guest, or a network component etc. - each one will have different schemas so we need
    // specific code to extract the name to log
    // for example:
    // {"instruction":{"Destroy":[{"Network":{"Ovn":{"RouterPort":{"name":"avd-lr0-sw0","parent_router":"avd-lr0" ...
    // {"instruction":{"Destroy":[{"Guest":{"name":"nginx" ...

    let name;
    switch (true) {
        case "Guest" in resource_json:
            name = resource_json["Guest"]["name"];
            break;
        case "Network" in resource_json:
            // for now, assume it is always OVN network, in the future we might have different networks so will need to
            // add another level of switch statement
            name = "Ovn ";
            let context = "";
            switch (true) {
                case "Switch" in resource_json["Network"]["Ovn"]:
                    name = name + "Switch " + resource_json["Network"]["Ovn"]["Switch"]["name"];
                    break;
                case "SwitchPort" in resource_json["Network"]["Ovn"]:
                    name = name + "Switch Port " + resource_json["Network"]["Ovn"]["SwitchPort"]["name"];
                    break;
                case "Router" in resource_json["Network"]["Ovn"]:
                    name = name + "Router " + resource_json["Network"]["Ovn"]["Router"]["name"];
                    break;
                case "RouterPort" in resource_json["Network"]["Ovn"]:
                    name = name + "Router Port " + resource_json["Network"]["Ovn"]["RouterPort"]["name"];
                    break;
                case "Route" in resource_json["Network"]["Ovn"]:
                    context = "(Subnet: " + resource_json["Network"]["Ovn"]["Route"]["prefix"]["Subnet"]["ip"] + "/" + resource_json["Network"]["Ovn"]["Route"]["prefix"]["Subnet"]["mask"] + ", Gateway: " + resource_json["Network"]["Ovn"]["Route"]["next_hop"]["Ip"] + ") on LR " + resource_json["Network"]["Ovn"]["Route"]["router_name"];
                    name = name + "Static Route " + context;
                    break;
                case "Nat" in resource_json["Network"]["Ovn"]:
                    let nat = resource_json["Network"]["Ovn"]["Nat"];
                    context = "(Ip: " + nat["external_ip"]["Ip"] + ", Subnet: " + nat["logical_ip"]["Subnet"]["ip"] + "/" + nat["logical_ip"]["Subnet"]["mask"] + ", " + nat["nat_type"] + ") on LR " + nat["logical_router_name"];
                    name = name + "Nat Rule " + context;
                    break;
                case "DhcpOption" in resource_json["Network"]["Ovn"]:
                    let dhcp = resource_json["Network"]["Ovn"]["DhcpOption"];
                    context = "(cidr: " + dhcp["cidr"]["Subnet"]["ip"] + "/" + dhcp["cidr"]["Subnet"]["mask"] + ", router: " + dhcp["router"] + ")"
                    name = name + "DHCP Option rule " + context;
                    break;
                case "ExternalGateway" in resource_json["Network"]["Ovn"]:
                    let ext_gateway = resource_json["Network"]["Ovn"]["ExternalGateway"];
                    context = "(" + ext_gateway["router_port_name"] + ", " + ext_gateway["chassis_name"] + ") on LRP ovn-lr0-public";
                    name = name + "External Gateway " + context;
                    break;
                // TODO - any others?
            }
            break;
    }

    return name;
}
