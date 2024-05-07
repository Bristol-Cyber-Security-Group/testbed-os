$(document).ready(function() {

    let validation_yaml_badge = $('#yamlValidationBadge');
    let validation_project_badge = $('#projectValidationBadge');

    $('#validateYamlButton').click(function (event) {
        $.ajax({
            url: server_url + '/api/validate/yaml',
            type: 'POST',
            data: $('#projectYamlText').val(),
            success: function (result_state) {
                console.log(result_state);

                validation_yaml_badge.removeClass("text-bg-secondary");
                validation_yaml_badge.removeClass("text-bg-danger");
                validation_yaml_badge.addClass("text-bg-success");
                validation_yaml_badge.html('Valid Yaml');
            },
            error: function (error) {
                console.log('was not able validate yaml: ' + error.responseText);

                validation_yaml_badge.removeClass("text-bg-secondary");
                validation_yaml_badge.removeClass("text-bg-success");
                validation_yaml_badge.addClass("text-bg-danger");
                validation_yaml_badge.html('Invalid Yaml');

                alert(error.responseText);
            }
        });
    });

    $('#validateProjectNameButton').click(function (event) {
        let input_name = $('#projectNameText').val();
        let body = {
            project_name: input_name,
        };
        $.ajax({
            url: server_url + '/api/validate/projectname',
            type: 'POST',
            data: JSON.stringify(body),
            contentType: 'application/json',
            success: function (result_state) {
                console.log(result_state);

                validation_project_badge.removeClass("text-bg-secondary");
                validation_project_badge.removeClass("text-bg-danger");
                validation_project_badge.addClass("text-bg-success");
                validation_project_badge.html('Valid Deployment Name');
            },
            error: function (error) {
                console.log('was not able validate project name: ' + error.responseText);

                validation_project_badge.removeClass("text-bg-secondary");
                validation_project_badge.removeClass("text-bg-success");
                validation_project_badge.addClass("text-bg-danger");
                validation_project_badge.html(error.responseText);

                alert(error.responseText);
            }
        });
    });

    $('#createDeploymentButton').click(function (event) {
        let input_name = $('#projectNameText').val();
        let input_yaml = $('#projectYamlText').val();
        let body = {
            project_name: input_name,
            yaml: input_yaml,
        };
        $.ajax({
            url: server_url + '/gui/deployments/create',
            type: 'POST',
            data: JSON.stringify(body),
            contentType: 'application/json',
            success: function (result_state) {
                console.log(result_state);
                // TODO change modal text

                $('#deploymentCreateSuccessModalText').html('Creating deployment ' + input_name + ' was successful.');
                $('#deploymentCreateSuccessModalLink').attr("href", "/gui/deployments/" + input_name);
                $('#deploymentCreateSuccessModal').modal('show');

            },
            error: function (error) {
                console.log('was not able validate project name: ' + error.responseText);


                alert(error.responseText);
            }
        });
    });

});