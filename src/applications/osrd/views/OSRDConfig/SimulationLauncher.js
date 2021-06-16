import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useDispatch, useSelector } from 'react-redux';
import nextId from 'react-id-generator';
import { post } from 'common/requests';
import { updateName, eraseSimulation } from 'reducers/osrdconf';
import {
  redirectToGraph, updateSimulation, toggleWorkingStatus,
} from 'reducers/osrdsimulation';
import { Redirect } from 'react-router-dom';
import InputSNCF from 'common/BootstrapSNCF/InputSNCF';
import AlertSNCF from 'common/BootstrapSNCF/AlertSNCF';
import ModalSNCF from 'common/BootstrapSNCF/ModalSNCF/ModalSNCF';
import ModalBodySNCF from 'common/BootstrapSNCF/ModalSNCF/ModalBodySNCF';

import formatConf from 'applications/osrd/components/SimulationLauncher/formatConf';

const osrdURI = '/osrd/simulate';
const scheduleURL = '/osrd/train_schedule'

export default function SimulationLauncher() {
  const [errorMessagesState, setErrorMessages] = useState([]);
  const osrdconf = useSelector((state) => state.osrdconf);
  const osrdsimulation = useSelector((state) => state.osrdsimulation);
  const { t } = useTranslation(['translation', 'osrdconf']);
  const dispatch = useDispatch();

  const submitConf = async () => {
    const osrdConfig = formatConf(setErrorMessages, t, osrdconf);

    if (osrdConfig !== false) {
      try {
        // store.dispatch(redirectToGraph(true));
        // store.dispatch(toggleWorkingStatus(true));
      //  const osrdReturn = await post(osrdURI, osrdConfig);
        console.log(osrdConfig);
        // store.dispatch(updateSimulation(osrdReturn));
        // store.dispatch(toggleWorkingStatus(false));
      } catch (e) {
        console.log(e);
      }
    }
  };

  return (
    <>
      {osrdsimulation.redirectToGraph ? <Redirect to="/osrd/graph" /> : ''}
      <div className="osrd-config-item">
        <div className="osrd-config-item-container d-flex align-items-center mb-2 bg-gray">
          <div className="lead font-weight-bold text-white">{osrdconf.name}</div>
          <button className="btn btn-sm btn-danger ml-auto mr-2" type="button" data-toggle="modal" data-target="#modal-erase-simulation">
            {t('osrdconf:eraseSimulation')}
          </button>
          <button className="btn btn-sm btn-primary" type="button" onClick={submitConf}>
            {t('osrdconf:launchSimulation')}
          </button>
        </div>
        <div className="osrd-config-item-container d-flex align-items-end mb-2">
          <InputSNCF
            type="text"
            label={t('osrdconf:name')}
            id="osrdconf-name"
            onChange={(e) => dispatch(updateName(e.target.value))}
            value={osrdconf.name}
            noMargin
            sm
          />
        </div>
        {errorMessagesState.length > 0
          ? (
            <AlertSNCF title={t('translation:common.error')}>
              <ul className="mt-1 mb-0">
                {errorMessagesState.map((message) => <li key={nextId()}>{message}</li>)}
              </ul>
            </AlertSNCF>
          ) : null}
      </div>
      <ModalSNCF htmlID="modal-erase-simulation" size="sm">
        <ModalBodySNCF>
          <>
            <div className="mb-3 h1">{t('osrdconf:eraseSimulationConfirm')}</div>
            <div className="d-flex">
              <button className="btn btn-secondary flex-fill mr-2" type="button" data-dismiss="modal">
                {t('translation:common.cancel')}
              </button>
              <button className="btn btn-danger flex-fill" type="button" data-dismiss="modal" onClick={() => store.dispatch(eraseSimulation())}>
                {t('translation:common.erase')}
              </button>
            </div>
          </>
        </ModalBodySNCF>
      </ModalSNCF>
    </>
  );
}
