$(document).ready(function() {

    $('#hostConfigText').load(server_url + '/api/config/host?pretty=true');

    $('#clusterConfigText').load(server_url + '/api/config/cluster?pretty=true');

    // TODO - poll the cluster configuration periodically in case a testbed joins/leaves in the background

    // TODO - update the host config with what is inside the text box

});