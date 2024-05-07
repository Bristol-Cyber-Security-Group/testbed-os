$(document).ready(function() {

    // fill the yaml collapse group
    $('#yamlTextBox').load(server_url + '/api/deployments/' + project_name + '/yaml');

    // update yaml button
    $('#updateYamlButton').click(function () {
        // validate the yaml
        $.ajax({
            url: server_url + '/api/validate/yaml',
            type: 'POST',
            data: $('#yamlTextBox').val(),
            success: function (result_state) {
                // push yaml if Ok
                $.ajax({
                    url: server_url + '/gui/deployments/' + project_name + '/yaml',
                    type: 'POST',
                    data: $('#yamlTextBox').val(),
                    success: function (result_state) {
                        // update textbox
                        $('#yamlTextBox').load(server_url + '/api/deployments/' + project_name + '/yaml');
                    },
                    error: function (error) {
                        console.log(error);
                        alert('error updating yaml' + error);
                    }
                });
            },
            error: function (error) {

                alert(error.responseText);
            }
        });
    })

    // fill the state json collapse group
    $('#stateJsonTextBox').load(server_url + '/api/deployments/' + project_name + '/state?pretty=true');

    // wire up the delete deployment button
    $('#deleteDeploymentButton').click(function () {
        $.ajax({
            url: server_url + '/gui/deployments/' + project_name + '/delete',
            type: 'POST',
            data: "[]",
            success: function (data) {
                window.location.href = server_url + "/gui/deployments";
            },
            error: function (error) {
                alert("something went wrong with the delete:" + error);
            }
        });
    });
    // TODO - format these files so that they aren't squeezed as if they were generic HTML text

    // fill the network topology collapse group
    fetch(`${server_url}/api/deployments/${project_name}/state`)
        .then(response => response.json())
        .then(stateData => {
            topologyData = parseStateData(stateData);
            drawTopology(topologyData);
        })
        .catch(error => console.error('Error fetching state.json:', error));
});

function parseStateData(stateData) {
    var nodesArray = [];
    var edgesArray = [];


    // Add master node
    if (stateData.testbed_hosts) {
        for (var hostKey in stateData.testbed_hosts) {
            var host = stateData.testbed_hosts[hostKey]; 
            nodesArray.push({
                id: hostKey, 
                label: hostKey.charAt(0).toUpperCase() + hostKey.slice(1) + ' Host', 
                ip: host.ip, 
                testbed_nic: host.testbed_nic, 
                shape: 'box' 
            });
        }
    }

    // Add switches
    if (stateData.network.ovn.switches) {
        for (var sw in stateData.network.ovn.switches) {
            var switchData = stateData.network.ovn.switches[sw];
            nodesArray.push({id: sw, label: switchData.name, shape: 'diamond'});
        }
    }

    // Add guest nodes
    if (stateData.testbed_guests) {
        for (var guest in stateData.testbed_guests) {
            var guestData = stateData.testbed_guests[guest];
            nodesArray.push({
                id: guest, 
                label: guest.charAt(0).toUpperCase() + guest.slice(1),
                ip: guestData.network.ip, 
                mac: guestData.network.mac 
            });
        }
    }

    // Add routers
    if (stateData.network.ovn.routers) {
        for (var router in stateData.network.ovn.routers) {
            var routerData = stateData.network.ovn.routers[router];
            nodesArray.push({id: router, label: routerData.name, shape: 'triangle'});
        }
    }

    // Add connections for routers
    if (stateData.network.ovn.router_ports) {
        for (var portName in stateData.network.ovn.router_ports) {
            var portData = stateData.network.ovn.router_ports[portName];
            var parentRouter = portData.parent_router;
    
            // Assuming the port naming convention 'deploymentName-routerName-switchName'
            var target = portName.split('-')[portName.split('-').length - 1];

            var switchNode = nodesArray.find(node => node.shape === 'diamond' && node.label.includes(target));
            if (switchNode) {
                edgesArray.push({from: parentRouter, to: switchNode.id});
            } else {
                console.log("No matching switch found for router:", parentRouter);
            }
        }
    }

    // Add connections for switches
    if (stateData.network.ovn.switch_ports) {
        for (var portName in stateData.network.ovn.switch_ports) {
            var portData = stateData.network.ovn.switch_ports[portName];
            var parentSwitch = portData.parent_switch;

            // Assuming the naming convention 'switchName-DeviceName'
            var parts = portName.split('-');
            var targetDevice = parts[parts.length - 1];

            var switchNode = nodesArray.find(node => node.id === parentSwitch);
            var deviceNode = nodesArray.find(node => node.id === targetDevice || node.id === portName);

            if (switchNode && deviceNode) {
                console.log("adding connection for ", switchNode.id + deviceNode.id);
                edgesArray.push({from: switchNode.id, to: deviceNode.id});
            } else {
                console.log("No matching device found for switch port:", portName);
            }
        }
    }

    var nodes = new vis.DataSet(nodesArray);
    var edges = new vis.DataSet(edgesArray);
    var data = {
        nodes: nodes,
        edges: edges
    };

    return data;
}

function drawTopology(data) {

    var container = document.getElementById('topologydiv');

    var options = {
        edges: {
            smooth: false
        },
        layout: {
            hierarchical: {
                enabled: true,
                levelSeparation: 150,
                nodeSpacing: 150,
                treeSpacing: 200,
                direction: 'UD',
                sortMethod: 'hubsize'
            }
        },
        physics: true
    };

    var network = new vis.Network(container, data, options);

    // Set up observer to fit network *only after* DOM has loaded the topology div
    var observer = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            var container = document.getElementById('topologydiv');
            if (container.offsetWidth > 0 && container.offsetHeight > 0) {
                network.fit();
                observer.disconnect();
            }
        });
    });
    var config = { attributes: true, childList: true, subtree: true };
    observer.observe(document.body, config);
    

    // Show info if node clicked
    network.on("click", function (params) {
        if (params.nodes.length > 0) {
            var nodeId = params.nodes[0];
            var nodeInfo = data.nodes.get(nodeId);
            var infoContent = '<strong>Node ID:</strong> ' + nodeId + '<br>' +
                            '<strong>Label:</strong> ' + nodeInfo.label + '<br>';

            if (nodeInfo.ip) {
                infoContent += '<strong>IP:</strong> ' + nodeInfo.ip + '<br>';
            }
            if (nodeInfo.mac) {
                infoContent += '<strong>MAC:</strong> ' + nodeInfo.mac + '<br>';
            }
            if (nodeInfo.testbed_nic) {
                infoContent += '<strong>Testbed NIC:</strong> ' + nodeInfo.testbed_nic;
            }
            
            var infoBox = document.getElementById('nodeInfoBox');
            infoBox.innerHTML = infoContent;
            
            var infoBox = document.getElementById('nodeInfoBox');
            infoBox.innerHTML = infoContent;
    
            var canvasPos = params.pointer.DOM;
            infoBox.style.top = canvasPos.y + 'px';
            infoBox.style.left = canvasPos.x + 'px';
            infoBox.style.display = 'block';
        } else {
            document.getElementById('nodeInfoBox').style.display = 'none';
        }
    });
    
}
